# ptp-import
Picture Transport Protocol (PTP) file importer. Put images into folders based on their capture date.

### Example Import
#### Camera Contents
```
/DCIM
  /100_FUJI
    DSCF0001.JPG (20/05/2020)
    DSCF0002.JPG (20/05/2020)
    DSCF0003.JPG (22/05/2020)
    DSCF0004.JPG (03/06/2020)
    DSCF0005.JPG (04/06/2020)
    DSCF0006.JPG (04/06/2020)
```
#### Resulting Files after Import
```
/2020
  /05
    /20
      DSCF0001.JPG
      DSCF0002.JPG
    /22
      DSCF0003.JPG
  /06
    /03
      DSCF0004.JPG
    /04
      DSCF0005.JPG
      DSCF0006.JPG
```

### Duplicates
Any duplicate images will be ignored. If the filenames are the same, but the contents are different, the file will be
given a duplicate name. E.g. `DSCF001.jpg` would become `DSCF001-1.jpg`.