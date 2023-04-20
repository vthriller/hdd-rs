/*!
Module to parse and manipulate attribute descriptions.

Attribute descriptions usually come from two sources:

* [drivedb.h](../index.html) entries,
* command-line arguments (`smartctl -v …`).

Format for attribute descriptions is described in [smartctl(8)](https://www.smartmontools.org/browser/trunk/smartmontools/smartctl.8.in) (option `-v`/`--vendorattribute`).
*/
use std::str;

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till1},
    character::complete::{char, digit1},
    combinator::{complete, eof, map, map_res, opt, value},
    sequence::preceded,
    IResult,
};

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Parse {
            // TODO? Parse(nom::verbose_errors::Err) if dependencies.nom.features = ["verbose-errors"]
            display("Unable to parse vendor attribute")
        }
    }
}

/// HDD or SSD
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type {
    HDD,
    SSD,
}

/// SMART attribute description
#[derive(Debug, Clone)]
pub struct Attribute {
    /// id of described attribute
    pub id: Option<u8>,
    /// attribute name
    pub name: Option<String>,
    /// value format, like `raw48` or `tempminmax`
    pub format: String,
    /// bytes of attribute data to make value of (usually something like `r543210`, where `r`, `v`, `w` represent reserved byte, current and worst values respectively)
    pub byte_order: String,
    /// what kind of device this description is applicable to: HDD, SSD, or both
    pub drivetype: Option<Type>,
}

fn not_comma(c: u8) -> bool {
    c == b','
}
fn not_comma_nor_colon(c: u8) -> bool {
    c == b',' || c == b':'
}

// `opt!()` is used with `complete!()` here because the former returns `Incomplete` untouched, thus making attributes not ending with otherwise optional ',(HDD|SSD)' `Incomplete` as well.
fn parse_standard(i: &[u8]) -> IResult<&[u8], Attribute> {
    let (i, id) = alt((
        // XXX map_res!()?
        map(digit1, |x: &[u8]| {
            str::from_utf8(x).unwrap().parse::<u8>().ok()
        }),
        value(None, char('N')),
    ))(i)?;

    let (i, _) = char(',')(i)?;
    let (i, format) = map_res(take_till1(not_comma_nor_colon), str::from_utf8)(i)?;
    // TODO '+' for ATTRFLAG_INCREASING
    let (i, byte_order) = opt(complete(preceded(
        char(':'),
        // TODO len()<6 should be invalid
        map_res(take_till1(not_comma), str::from_utf8),
    )))(i)?;

    fn parse_name_drive_type(i: &[u8]) -> IResult<&[u8], (&str, Option<Type>)> {
        let (i, _) = char(',')(i)?;
        let (i, name) = map_res(take_till1(not_comma), str::from_utf8)(i)?;
        let (i, drive_type) = opt(complete(map(
            preceded(char(','), alt((tag("HDD"), tag("SSD")))),
            |drive_type| match str::from_utf8(drive_type) {
                Ok("HDD") => Type::HDD,
                Ok("SSD") => Type::SSD,
                _ => unreachable!(),
            },
        )))(i)?;

        Ok((i, (name, drive_type)))
    }
    let (i, name_drive_type) = opt(complete(parse_name_drive_type))(i)?;
    let (i, _) = eof(i)?;

    let (name, drive_type) = match name_drive_type {
        Some((name, drive_type)) => (Some(name), drive_type),
        None => (None, None),
    };
    let default_byte_order = match format {
        // default byte orders, from ata_get_attr_raw_value, atacmds.cpp
        "raw64" | "hex64" => "543210wv",
        "raw56" | "hex56" | "raw24/raw32" | "msec24hour32" => "r543210",
        _ => "543210",
    };
    Ok((
        i,
        Attribute {
            id: id,
            name: name.map(|x| x.to_string()),
            format: format.to_string(),
            byte_order: byte_order.unwrap_or(default_byte_order).to_string(),
            drivetype: drive_type,
        },
    ))
}

/**
Parses single attribute description (`-v` option argument).

The following formats are supported:

* `ID,FORMAT[:BYTEORDER][,NAME[,(HDD|SSD)]]`
* legacy `-v` arguments, like `9,halfminutes`
*/
pub fn parse(s: &str) -> Result<Attribute, Error> {
    let s = match s {
        "9,halfminutes" => "9,halfmin2hour,Power_On_Half_Minutes",
        "9,minutes" => "9,min2hour,Power_On_Minutes",
        "9,seconds" => "9,sec2hour,Power_On_Seconds",
        "9,temp" => "9,tempminmax,Temperature_Celsius",
        "192,emergencyretractcyclect" => "192,raw48,Emerg_Retract_Cycle_Ct",
        "193,loadunload" => "193,raw24/raw24",
        "194,10xCelsius" => "194,temp10x,Temperature_Celsius_x10",
        "194,unknown" => "194,raw48,Unknown_Attribute",
        "197,increasing" => "197,raw48+,Total_Pending_Sectors",
        "198,offlinescanuncsectorct" => "198,raw48,Offline_Scan_UNC_SectCt", // TODO? there goes some `get_unc_attr_id` reference
        "198,increasing" => "198,raw48+,Total_Offl_Uncorrectabl",
        "200,writeerrorcount" => "200,raw48,Write_Error_Count",
        "201,detectedtacount" => "201,raw48,Detected_TA_Count",
        "220,temp" => "220,tempminmax,Temperature_Celsius",
        s => s,
    };
    // FIXME strings to bytes to strings again… sounds really stupid
    match parse_standard(s.as_bytes()) {
        Ok((_, attr)) => Ok(attr),
        Err(nom::Err::Error(_)) => Err(Error::Parse), // TODO?
        Err(nom::Err::Incomplete(_)) => Err(Error::Parse), // TODO?
        Err(nom::Err::Failure(_)) => todo!(),         // TODO?
    }
}
