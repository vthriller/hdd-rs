use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{char, multispace0, none_of, one_of},
    combinator::{eof, map, value},
    multi::{many0, many1},
    sequence::tuple,
    IResult,
};

fn comment_block(input: &[u8]) -> IResult<&[u8], ()> {
    value(
        (), // output is thrown away
        tuple((tag("/*"), take_until("*/"), tag("*/"))),
    )(input)
}

fn comment(input: &[u8]) -> IResult<&[u8], ()> {
    value(
        (), // output is thrown away
        tuple((tag("//"), take_until("\n"), tag("\n"))),
    )(input)
}

fn whitespace(input: &[u8]) -> IResult<&[u8], ()> {
    value(
        (), // output is thrown away
        many0(alt((value((), multispace0), comment, comment_block))),
    )(input)
}

// TODO? \[bfav?0] \ooo \xhh
fn string_escaped_char(i: &[u8]) -> IResult<&[u8], char> {
    let (i, _) = char('\\')(i)?;
    let (i, s) = map(one_of("\\\"'nrt"), |c| match c {
        '\\' => '\\',
        '"' => '"',
        '\'' => '\'',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        _ => unreachable!(),
    })(i)?;
    Ok((i, s))
}

fn string_char(i: &[u8]) -> IResult<&[u8], char> {
    alt((none_of("\n\\\""), string_escaped_char))(i)
}

fn string_literal(i: &[u8]) -> IResult<&[u8], String> {
    let (i, _) = char('\"')(i)?;
    let (i, s) = map(many0(string_char), |s: Vec<char>| s.into_iter().collect())(i)?;
    let (i, _) = char('\"')(i)?;
    Ok((i, s))
}

fn string(i: &[u8]) -> IResult<&[u8], String> {
    let (i, s0) = string_literal(i)?;
    fn parse_ss(i: &[u8]) -> IResult<&[u8], String> {
        let (i, _) = whitespace(i)?;
        let (i, s) = string_literal(i)?;
        Ok((i, s))
    }
    let (i, ss) = many0(parse_ss)(i)?;

    let mut out = s0;
    for s in ss {
        out.push_str(s.as_str())
    }
    Ok((i, out))
}

/// drivedb.h entry
#[derive(Debug)]
pub struct Entry {
    /// > Informal string about the model family/series of a device.
    pub family: String,

    /// > POSIX extended regular expression to match the model of a device.
    /// > This should never be "".
    pub model: String,

    /// > POSIX extended regular expression to match a devices's firmware.
    ///
    /// Optional if "".
    pub firmware: String,

    /// > A message that may be displayed for matching drives.
    /// > For example, to inform the user that they may need to apply a firmware patch.
    pub warning: String,

    /// > String with vendor-specific attribute ('-v') and firmware bug fix ('-F') options.
    /// > Same syntax as in smartctl command line.
    pub presets: String,
}

fn comma(i: &[u8]) -> IResult<&[u8], ()> {
    value(
        (), // output is thrown away
        tuple((whitespace, char(','), whitespace)),
    )(i)
}

fn entry(i: &[u8]) -> IResult<&[u8], Entry> {
    let (i, _) = char('{')(i)?;
    let (i, _) = whitespace(i)?;
    let (i, family) = string(i)?;
    let (i, _) = comma(i)?;
    let (i, model) = string(i)?;
    let (i, _) = comma(i)?;
    let (i, firmware) = string(i)?;
    let (i, _) = comma(i)?;
    let (i, warning) = string(i)?;
    let (i, _) = comma(i)?;
    let (i, presets) = string(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = char('}')(i)?;
    Ok((
        i,
        Entry {
            family,
            model,
            firmware,
            warning,
            presets,
        },
    ))
}

pub fn database(i: &[u8]) -> IResult<&[u8], Vec<Entry>> {
    fn parse_entry(i: &[u8]) -> IResult<&[u8], Entry> {
        let (i, entry) = entry(i)?;
        let (i, _) = comma(i)?;
        Ok((i, entry))
    }
    let (i, _) = whitespace(i)?;
    let (i, entries) = many1(parse_entry)(i)?;
    let (i, _) = whitespace(i)?;
    let (i, _) = eof(i)?;

    Ok((
        i,
        entries
            .into_iter()
            .filter(|entry| {
                // > The entry is ignored if [modelfamily] starts with a dollar sign.
                !entry.family.starts_with('$')
            })
            .collect(),
    ))
}
