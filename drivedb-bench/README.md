# Overview

Legend:

1. find()
2. precompiled
3. LazyRegex
4. RegexSet

## Debug

|               | Best first disk  | Best consecutive disks
|---------------|------------------|-----------------------
| Best case     | 1, 3, 4, …, 2    | 2/3, 4, 1
| Worst case    | 4, …, 1/2, 3     | 2/3, 4, …, 1
| HGST          | 4, …, 1, 3, …, 2 | 2/3, 4, …, 1
| Samsung SSD   | 4, 1, 3, …, 2    | 2/3, 4, …, 1

Without find(), 5s results:

|               | Best first disk  | Best consecutive disks
|---------------|------------------|-----------------------
| Best case     | 3, 4             | 2/3, 4
| Worst case    | 4                | 2/3, 4
| HGST          | 4, …, 3          | 2/3, 4
| Samsung SSD   | 4, 3             | 2/3, 4

## Release

|               | Best first disk  | Best consecutive disks
|---------------|------------------|-----------------------
| Best case     | 1, 3, 4, 2       | 2/3, 4, 1
| Worst case    | 4, 1, 2/3        | 2/3, 4, 1
| HGST          | 4, 1, 3, 2       | 2/3, 4, 1
| Samsung SSD   | 4, 1, 3, 2       | 2/3, 4, 1

## Conclusion

- find(): obviously bad (except for the matches closest to the top of the drivedb). Only included for comparison.
- precompiled: terrible startup times, obviously. Especially in debug releases.
- LazyRegex: also relatively easy to plug into existing code, makes lots of cases better, but not all of them. It also comes from an extra dependency.
- RegexSet: surprisingly this turned out to be the best solution for most of the first-disk cases, on par with other top-performers in all other cases. This is especially cool considering [there's probably some room for improvement](https://github.com/rust-lang/regex/issues/259).

# Tests

<!--
>>> from statistics import median, pstdev, mean
>>> A = [36.8720,…]
>>> print('%.4f         | %.4f | %.4f | %.4f | %.4f' % (min(A), mean(A), max(A), median(A), pstdev(A)))
-->

## Best case

`drivedb-bench ./drivedb.h '2GB SATA Flash Drive' SFDDA01C`

#### Debug

|             | compilation | first   | consecutive min | avg     | max     | median  | pstddev
|-------------|-------------|---------|-----------------|---------|---------|---------|--------
| find()      | -           | 38.7481 | 35.4281         | 36.3299 | 39.3407 | 35.7713 | 1.1937
| precompiled | 5227.1035   | 0.7380  | 0.0109          | 0.0124  | 0.0146  | 0.0118  | 0.0012
| LazyRegex   | 148.4984    | 35.6199 | 0.0098          | 0.0117  | 0.0156  | 0.0101  | 0.0025
| RegexSet    | 196.2486    | 38.5432 | 2.9315          | 3.0512  | 3.3947  | 3.0054  | 0.1359

#### Release

|             | compilation | first   | consecutive min | avg     | max     | median  | pstddev
|-------------|-------------|---------|-----------------|---------|---------|---------|--------
| find()      | -           | 1.1759  | 0.7647          | 0.7954  | 0.9174  | 0.7778  | 0.0447
| precompiled | 135.8502    | 0.0682  | 0.0003          | 0.0006  | 0.0020  | 0.0005  | 0.0005
| LazyRegex   | 9.0409      | 0.8632  | 0.0003          | 0.0007  | 0.0016  | 0.0006  | 0.0004
| RegexSet    | 20.3582     | 1.7935  | 0.1077          | 0.1178  | 0.1284  | 0.1164  | 0.0078

## Worst case

`drivedb-bench ./drivedb.h ultramegaunknown zzz`

#### Debug

|             | compilation | first     | consecutive min | avg       | max       | median    | pstddev
|-------------|-------------|-----------|-----------------|-----------|-----------|-----------|--------
| find()      | -           | 5587.3901 | 5166.1274       | 5290.0926 | 5479.7534 | 5255.1777 | 99.2660
| precompiled | 5247.6421   | 49.3055   | 0.5865          | 0.7552    | 0.8685    | 0.7883    | 0.0932
| LazyRegex   | 153.8509    | 5322.1274 | 0.8106          | 0.8638    | 1.0859    | 0.8246    | 0.0834
| RegexSet    | 202.4601    | 22.2448   | 2.8498          | 2.9753    | 3.3019    | 2.9156    | 0.1495

#### Release

|             | compilation | first    | consecutive min | avg      | max      | median   | pstddev
|-------------|-------------|----------|-----------------|----------|----------|----------|--------
| find()      | -           | 120.8510 | 118.5161        | 130.2581 | 182.0322 | 123.2989 | 19.0847
| precompiled | 139.3060    | 3.0567   | 0.0401          | 0.0507   | 0.1132   | 0.0424   | 0.0223
| LazyRegex   | 8.6232      | 141.0907 | 0.0613          | 0.0817   | 0.2126   | 0.0646   | 0.0465
| RegexSet    | 20.0097     | 1.1399   | 0.0958          | 0.1096   | 0.1268   | 0.1102   | 0.0086

## Other models

### `drivedb-bench ./drivedb.h 'HGST HUS724020ALA640' MF6OAA70`

#### Debug

|             | compilation | first      | consecutive min | avg       | max       | median    | pstddev
|-------------|-------------|------------|-----------------|-----------|-----------|-----------|--------
| find()      | -           | 2752.0298  | 2710.4380       | 2744.7275 | 2800.7295 | 2744.7920 | 30.9750
| precompiled | 5236.5552   | 23.5294    | 0.3245          | 0.3668    | 0.4718    | 0.3491    | 0.0458
| LazyRegex   | 159.3884    | 2840.0708  | 0.3609          | 0.3929    | 0.4450    | 0.3849    | 0.0264
| RegexSet    | 190.9953    | 37.3394    | 3.0535          | 3.7801    | 5.0103    | 3.1188    | 0.8176

#### Release

|             | compilation | first    | consecutive min | avg      | max      | median   | pstddev
|-------------|-------------|----------|-----------------|----------|----------|----------|--------
| find()      | -           | 62.7819  | 61.4845         | 62.2971  | 63.2365  | 62.0446  | 0.6209
| precompiled | 138.9426    | 1.7134   | 0.0218          | 0.0284   | 0.0451   | 0.0271   | 0.0062
| LazyRegex   | 8.7305      | 74.1788  | 0.0228          | 0.0410   | 0.1282   | 0.0328   | 0.0313
| RegexSet    | 20.8532     | 1.8317   | 0.1014          | 0.1139   | 0.1280   | 0.1163   | 0.0090

### `drivedb-bench ./drivedb.h 'Samsung SSD 750 EVO 250GB' MAT01B6Q`

#### Debug

|             | compilation | first      | consecutive min | avg       | max       | median    | pstddev
|-------------|-------------|------------|-----------------|-----------|-----------|-----------|--------
| find()      | -           | 336.4690   | 326.1739        | 345.2693  | 401.6722  | 330.8318  | 28.9980
| precompiled | 5203.6152   | 8.8471     | 0.0794          | 0.0866    | 0.1024    | 0.0841    | 0.0074
| LazyRegex   | 151.6676    | 378.5143   | 0.0932          | 0.1172    | 0.1723    | 0.0948    | 0.0282
| RegexSet    | 191.0855    | 43.7418    | 3.0022          | 3.1243    | 3.2391    | 3.1523    | 0.0716

#### Release

|             | compilation | first    | consecutive min | avg      | max      | median   | pstddev
|-------------|-------------|----------|-----------------|----------|----------|----------|--------
| find()      | -           | 10.3841  | 9.6405          | 9.9587   | 10.3235  | 9.8984   | 0.2106
| precompiled | 139.1604    | 0.5884   | 0.0035          | 0.0040   | 0.0070   | 0.0036   | 0.0011
| LazyRegex   | 8.9637      | 12.4214  | 0.0045          | 0.0066   | 0.0207   | 0.0046   | 0.0050
| RegexSet    | 22.1461     | 1.9978   | 0.1082          | 0.1381   | 0.1892   | 0.1324   | 0.0295
