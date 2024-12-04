use super::*;

#[test]
fn leb128_reader() {
    let data = [
        0x7F, 0xFF, 0x00, 0xEF, 0xFD, 0xB6, 0xF5, 0x0D, 0xEF, 0xFD, 0xB6, 0xF5, 0x7D,
    ];
    let mut reader = Leb128Reader::from_slice(&data);

    reader.reset();
    assert_eq!(reader.position(), 0);
    let test = reader.read_unsigned().unwrap();
    assert_eq!(test, 127);
    let test = reader.read_unsigned().unwrap();
    assert_eq!(test, 127);
    let test = reader.read_unsigned().unwrap();
    assert_eq!(test, 0xdeadbeef);
    let test = reader.read_unsigned().unwrap();
    assert_eq!(test, 0x7deadbeef);

    reader.reset();
    assert_eq!(reader.position(), 0);
    let test = reader.read_signed().unwrap();
    assert_eq!(test, -1);
    let test = reader.read_signed().unwrap();
    assert_eq!(test, 127);
    let test = reader.read_signed().unwrap();
    assert_eq!(test, 0xdeadbeef);
    let test = reader.read_signed().unwrap();
    assert_eq!(test, -559038737);

    let data = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0];
    let mut stream = Leb128Reader::from_slice(&data);

    stream.reset();
    assert_eq!(stream.position(), 0);
    let test = stream.read_unsigned().unwrap();
    assert_eq!(test, 0);
    assert_eq!(reader.read_byte().unwrap_err(), ReadError::UnexpectedEof);

    stream.reset();
    assert_eq!(stream.position(), 0);
    let test = stream.read_signed().unwrap();
    assert_eq!(test, 0);
    assert_eq!(reader.read_byte().unwrap_err(), ReadError::UnexpectedEof);
}

#[test]
fn leb128_writer() {
    let mut writer = Leb128Writer::new();

    writer.clear();
    assert_eq!(writer.len(), 0);
    writer.write(0u32).unwrap();
    assert_eq!(writer.as_slice(), &[0]);

    writer.clear();
    assert_eq!(writer.len(), 0);
    writer.write(0i32).unwrap();
    assert_eq!(writer.as_slice(), &[0]);

    for i in 0..64 {
        let value1 = 1u64 << i;
        let mut writer = Leb128Writer::new();
        writer.write(value1).unwrap();

        let byte_cnt = (i + 7) / 7;
        assert_eq!(writer.as_slice().len(), byte_cnt);

        assert_ne!(*writer.as_slice().last().unwrap(), 0);

        let mut reader = Leb128Reader::from_slice(writer.as_slice());
        let test1 = reader.read().unwrap();
        assert_eq!(value1, test1);
    }

    writer.clear();
    writer.write(127u32).unwrap();
    assert_eq!(writer.as_slice(), &[0x7F]);

    writer.clear();
    writer.write(128u32).unwrap();
    assert_eq!(writer.as_slice(), &[0x80, 0x01]);

    writer.clear();
    writer.write(0xdeadbeefu32).unwrap();
    assert_eq!(writer.as_slice(), &[0xEF, 0xFD, 0xB6, 0xF5, 0x0D]);

    writer.clear();
    writer.write(0x7deadbeefu64).unwrap();
    assert_eq!(writer.as_slice(), &[0xEF, 0xFD, 0xB6, 0xF5, 0x7D]);

    writer.clear();
    writer.write(127i32).unwrap();
    assert_eq!(writer.as_slice(), &[0xFF, 0x00]);

    writer.clear();
    writer.write(63i32).unwrap();
    assert_eq!(writer.as_slice(), &[0x3F]);

    writer.clear();
    writer.write(64i32).unwrap();
    assert_eq!(writer.as_slice(), &[0xC0, 0x00]);

    writer.clear();
    writer.write(-1i32).unwrap();
    assert_eq!(writer.as_slice(), &[0x7F]);

    writer.clear();
    writer.write(-64i32).unwrap();
    assert_eq!(writer.as_slice(), &[0x40]);

    writer.clear();
    writer.write(0xdeadbeefi64).unwrap();
    assert_eq!(writer.as_slice(), &[0xEF, 0xFD, 0xB6, 0xF5, 0x0D]);

    writer.clear();
    writer.write(-559038737i64).unwrap();
    assert_eq!(writer.as_slice(), &[0xEF, 0xFD, 0xB6, 0xF5, 0x7D]);

    writer.clear();
    writer.write(0x8000_0000u32).unwrap();
    assert_eq!(writer.as_slice(), &[0x80, 0x80, 0x80, 0x80, 0x08]);

    writer.clear();
    writer.write(-0x8000_0000i32).unwrap();
    assert_eq!(writer.as_slice(), &[0x80, 0x80, 0x80, 0x80, 0x78]);
}

#[test]
fn leb128_read_write() {
    for i in 0..64 {
        let value1u = 1u64 << i;
        let value2u = value1u - 1;
        let value3u = !value2u;

        let value1i = value1u as i64;
        let value2i = value2u as i64;
        let value3i = value3u as i64;

        let value5 = value2u & 0x5555_5555_5555_5555;
        let value6 = value2u & 0x1234_5678_9ABC_DEF0;
        let value7 = value2u & 0xDEAD_BEEF_F00D_BAAD;

        let mut writer = Leb128Writer::new();
        writer.write(value1i).unwrap();
        writer.write(value1u).unwrap();
        writer.write(value2i).unwrap();
        writer.write(value2u).unwrap();
        writer.write(value3i).unwrap();
        writer.write(value3u).unwrap();
        writer.write(value5).unwrap();
        writer.write(value6).unwrap();
        writer.write(value7).unwrap();
        let mut reader = Leb128Reader::from_slice(writer.as_slice());

        let test1i = reader.read().unwrap();
        assert_eq!(value1i, test1i);
        let test1u = reader.read().unwrap();
        assert_eq!(value1u, test1u);
        let test2i = reader.read().unwrap();
        assert_eq!(value2i, test2i);
        let test2u = reader.read().unwrap();
        assert_eq!(value2u, test2u);
        let test3i = reader.read().unwrap();
        assert_eq!(value3i, test3i);
        let test3u = reader.read().unwrap();
        assert_eq!(value3u, test3u);

        let test5 = reader.read().unwrap();
        assert_eq!(value5, test5);
        let test6 = reader.read().unwrap();
        assert_eq!(value6, test6);
        let test7 = reader.read().unwrap();
        assert_eq!(value7, test7);

        assert!(reader.is_eof());

        assert_eq!(reader.read_byte().unwrap_err(), ReadError::UnexpectedEof);
    }
}
