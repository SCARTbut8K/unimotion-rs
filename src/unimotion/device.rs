use macaddr::MacAddr6;
use base64::{Engine as _, alphabet, engine::{self, general_purpose}};

use std::str::FromStr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_si() -> Result<(), String> {
        let expected_id = 7;
        let expected_info = SensorInfo {
            sensor_version: 102,
            mystery_value: 78,
            mac_address: MacAddr6::from_str("08:3A:F2:6D:1D:98").unwrap(),
            channel: 1,
            tx_power: 10,
            datamode: 3,
            six_axis: false,
            imu_flip: true,
            min_mag_th: 0,
            max_mag_th: 124,
        };
        
        match Response::from("_si 7 Zk4IOvJtHZgBCloDAgAEAAAAASgIAHw=".as_bytes().to_vec()) {
            Response::SensorInfo(id, info) => {
                assert_eq!(expected_id, id);
                assert_eq!(expected_info, info);
            },
            r => return Err(format!(
                "Expected {:?} and received {:?}", Response::SensorInfo(expected_id, expected_info), r)
            )
        }
        Ok(())
    }

    #[test]
    fn test_dev() -> Result<(), String> {
        let addr1 = MacAddr6::from_str("E8:68:E7:53:55:DE");
        let addr2 = MacAddr6::nil();
        assert!(
            matches!(
                Response::from("_dev 2 E8 68 E7 53 55 DE".as_bytes().to_vec()), 
                Response::Device(2, addr1))
        );
        assert!(
            matches!(
                Response::from("_dev 23 0 0 0 0 0 0".as_bytes().to_vec()), 
                Response::Device(23, addr2))
        );
        Ok(())
    }

    #[test]
    fn test_ch() -> Result<(), String> {
        assert!(
            matches!(Response::from("_ch 1".as_bytes().to_vec()), Response::Channel(1))
        );
        assert!(
            matches!(Response::from("_ch 2".as_bytes().to_vec()), Response::Channel(2))
        );
        Ok(())
    }

    #[test]
    fn test_auto_off() -> Result<(), String> {
        assert!(
            matches!(Response::from("_auto_off 1 300000".as_bytes().to_vec()), Response::AutoOff(1, 300000))
        );
        Ok(())
    }

    #[test]
    fn test_ok() -> Result<(), String> {
        assert!(
            // vec![95, 111, 107, 13, 10] is "_ok\r\n"
            matches!(Response::from(vec![95, 111, 107, 13, 10]), Response::Acknowledge(AcknowledgeType::Alive))
        );
        assert!(
            matches!(Response::from("_ok ESP_RESTART".as_bytes().to_vec()), Response::Acknowledge(AcknowledgeType::RestartAP))
        );
        assert!(
            matches!(Response::from("_ok WIFI_ON".as_bytes().to_vec()), Response::Acknowledge(AcknowledgeType::StartWifi))
        );
        assert!(
            matches!(Response::from("_ok QUIT_CONFIG".as_bytes().to_vec()), Response::Acknowledge(AcknowledgeType::QuitConfig))
        );
        Ok(())
    }

    #[test]
    fn test_datamode() -> Result<(), String> {
        assert!(
            matches!(Response::from("_datamode 1".as_bytes().to_vec()), Response::Datamode(1))
        );
        assert!(
            matches!(Response::from("_datamode 3".as_bytes().to_vec()), Response::Datamode(3))
        );
        Ok(())
    }

    #[test]
    fn test_data() -> Result<(), String> {
        // B6cdte627NJ+Gxy1rbZs058bgP8
        // ---------------------------
        // 07
        // a7
        // 1d b5 ee b6 ec d2 7e 1b 
        // 1c b5 ad b6 6c d3 9f 1b
        // 80
        // ff
        let signed = |x: u16| x as i16;

        let expected_data = Datagram {
            id: 7,
            battery_voltage: 167,
            quaternions: [
                Some(Quaternion { x: signed(0x1b7e), y: signed(0xb6ee), z: -signed(0xd2ec), w: signed(0xb51d)}),
                Some(Quaternion { x: signed(0x1b9f), y: signed(0xb6ad), z: -signed(0xd36c), w: signed(0xb51c)}),
                None,
                None,
            ],
            ahrs_enable: 128,
            magnetic_power: 255,
        };

        match Response::from("B6cdte627NJ+Gxy1rbZs058bgP8".as_bytes().to_vec()) {
            Response::Data(data) => assert_eq!(expected_data, data),
            r => return Err(format!("Expected {:?} and received {:?}", Response::Data(expected_data), r))
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct UniSensorDevice {
    pub id: u8,
    pub mac_addr: MacAddr6,
    pub sensor_info: Option<SensorInfo>,
}

impl UniSensorDevice {
    pub fn empty() -> UniSensorDevice {
        UniSensorDevice {
            id: 255,
            mac_addr: MacAddr6::nil(),
            sensor_info: None,
        }
    }
}

#[derive(Debug)]
pub enum Response {
    SensorInfo(u8, SensorInfo),// _si
    Device(u8, MacAddr6),// _dev
    Channel(u8),// _ch
    AutoOff(u8, u64),// _auto_off
    Acknowledge(AcknowledgeType),// _ok
    Datamode(u8),// _datamode
    Data(Datagram),
    Error,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum AcknowledgeType {
    Alive,// ""
    RestartAP,// "ESP_RESTART"
    StartWifi,// "WIFI_ON"
    QuitConfig,// "QUIT_CONFIG"
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Quaternion {
    // bytes are sent as such:
    // W0 W1 Y0 Y1 Z0 Z1 X0 X1
    // 
    pub x: i16,//  X
    pub y: i16,//  Y
    pub z: i16,// -Z
    pub w: i16,//  W
}

impl From<[u8; 8]> for Quaternion {
    fn from(value: [u8; 8]) -> Self {
        let w = value[0] as u16 + ((value[1] as u16) << 8);
        let y = value[2] as u16 + ((value[3] as u16) << 8);
        let z = value[4] as u16 + ((value[5] as u16) << 8);
        let x = value[6] as u16 + ((value[7] as u16) << 8);

        Quaternion { x: x as i16, y: y as i16, z: -(z as i16), w: w as i16}
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Datagram {
    // I = id = 1byte
    // B = battery level = 1byte
    // Q = quaternion = 8bytes (4shorts)
    // A = ahrs enable = 1byte
    // M = magnetic level = 1byte
    // 
    // b64 | decoded |
    // ------------------------------------
    // 14  | 10.5    | IBQ
    // 16  | 12      | IBQAM
    // 24  | 18      | IBQQ
    // 27  | 20.25   | IBQQAM
    // 46  | 34.5    | IBQQQQ
    // 48  | 36      | IBQQQQAM
    // err | ???     | ???
    // 
    pub id: u8, // [0]
    pub battery_voltage: u8, // [1]
    pub quaternions: [Option<Quaternion>; 4], // [2]
    pub ahrs_enable: u8, // [10], [18], [34]
    pub magnetic_power: u8, // [11], [19], [35]
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct SensorInfo {
    // SensorVersion may be unknown
    sensor_version: u8, // [0]
    mystery_value: u8, // [1]
    // Note: This is the address of the station
    mac_address: MacAddr6, // [2..8]
    channel: u8,// [8]
	tx_power: u8,// [9]
	datamode: u8,// [11]
	six_axis: bool,// [17]
	imu_flip: bool,// [18]
    // SensorInfo response may not contain Magnetic thresolds
    min_mag_th: u8,// [21]
    max_mag_th: u8,// [22]
}

impl From<[u8; 19]> for SensorInfo {
    fn from(value: [u8; 19]) -> Self {
        // Minimum and maximum magnetic thresold are not transmitted
        let sensor_version = value[0];
        let mystery_value = value[1];
        // MAC address is set to nil in case a TryFrom::Error is encontered
        let mac_address = MacAddr6::from(value[2..8].try_into().unwrap_or([0; 6]));
        let channel = value[8];
        let tx_power  = value[9];
        let datamode  = value[11];
        let six_axis  = value[17] & 0x01 != 0;
        let imu_flip  = value[18] & 0x01 != 0;

        SensorInfo {
            sensor_version: sensor_version,
            mystery_value: mystery_value,
            mac_address: mac_address,
            channel: channel,
            tx_power: tx_power,
            datamode: datamode,
            six_axis: six_axis,
            imu_flip: imu_flip,
            min_mag_th: 255,
            max_mag_th: 255,
        }
    }
}

impl From<[u8; 23]> for SensorInfo {
    fn from(value: [u8; 23]) -> Self {
        let sensor_version = value[0];
        let mystery_value = value[1];
        // MAC address is set to nil in case a TryFrom::Error is encontered
        let mac_address = MacAddr6::from(value[2..8].try_into().unwrap_or([0; 6]));
        let channel = value[8];
        let tx_power  = value[9];
        let datamode  = value[11];
        let six_axis  = value[17] & 0x01 != 0;
        let imu_flip  = value[18] & 0x01 != 0;
        let min_mag_th  = value[21];
        let max_mag_th  = value[22];

        SensorInfo {
            sensor_version: sensor_version,
            mystery_value: mystery_value,
            mac_address: mac_address,
            channel: channel,
            tx_power: tx_power,
            datamode: datamode,
            six_axis: six_axis,
            imu_flip: imu_flip,
            min_mag_th: min_mag_th,
            max_mag_th: max_mag_th,
        }
    }
}

pub mod parsing {
    use super::*;

    fn parse(line: &str) -> Result<Response, &str> {
        let words: Vec<&str> = line.split(' ').collect();

        let res = match (words[0], line.len()) {
            ("_si", _) => {
                let (dev, sensor_info) = parse_si(&line["_si".len()..line.len()].trim_start())?;
                Response::SensorInfo(dev, sensor_info)
            },
            ("_dev", _) => {
                let (dev, addr) = parse_dev(&line["_dev".len()..line.len()].trim_start())?;
                Response::Device(dev, addr)
            },
            ("_ch", _) => {
                let ch = parse_ch(&line["_ch".len()..line.len()].trim_start())?;
                Response::Channel(ch)
            },
            ("_auto_off", _) => {
                let (enable, duration) = parse_auto_off(&line["_auto_off".len()..line.len()].trim_start())?;
                Response::AutoOff(enable, duration)
            },
            ("_ok", _) => {
                let ack = parse_ok(&line["_ok".len()..line.len()].trim_start())?;
                Response::Acknowledge(ack)
            },
            ("_datamode", _) => {
                let dm = parse_datamode(&line["_datamode".len()..line.len()].trim_start())?;
                Response::Datamode(dm)
            },
            // base64 encodes 6 bits for each byte (75%)
            // TODO: Elimintate repeated code with a macro?
            (_, 14) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(format!("{}==", line)) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 10]>>::parse_data(bytes[..10].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, 16) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(line) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 12]>>::parse_data(bytes[..12].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, 24) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(line) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 18]>>::parse_data(bytes[..18].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, 27) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(format!("{}=", line)) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 20]>>::parse_data(bytes[..20].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, 46) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(format!("{}==", line)) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 34]>>::parse_data(bytes[..34].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, 48) => {
                let Ok(bytes) = general_purpose::STANDARD.decode(line) else { return Err(line) };
                let data = <Datagram as Parseable<[u8; 36]>>::parse_data(bytes[..36].try_into().unwrap())?;
                Response::Data(data)
            },
            (_, _) => {
                return Err(line)
            },
        };
        Ok(res)
    }

    fn parse_si(line: &str) -> Result<(u8, SensorInfo), &str> {
        let v: Vec<&str> = line.split(' ').collect();
        if v.len() != 2 {
            return Err(line);
        }
        let Ok(id) = v[0].parse::<u8>() else { return Err(line) };

        let Ok(bytes) = general_purpose::STANDARD.decode(v[1]) else { return Err(line) };
        match bytes.len() {
            19 => {
                let bytes: [u8; 19] = bytes[..19].try_into().unwrap();
                Ok((id, SensorInfo::from(bytes)))
            },
            23 => {
                let bytes: [u8; 23] = bytes[..23].try_into().unwrap();
                Ok((id, SensorInfo::from(bytes)))
            },
            _ => {
                Err(line)
            }
        }
    }

    fn parse_dev(line: &str) -> Result<(u8, MacAddr6), &str> {
        let v: Vec<&str> = line.split(' ').collect();
        if v.len() != 7 {
            return Err(line);
        }
        let Ok(id) = v[0].parse::<u8>() else { return Err(line) };
        // Padding values because the unistation return 0 0 0 0 0 0 when no unisensor is set.
        let addr: &String = &v[1..7].iter().map(
            |byte| if byte.len() < 2 { format!("0{}", byte) } else { byte.to_string() }
        ).collect();
        let Ok(addr) = MacAddr6::from_str(&addr) else { return Err(line) };

        Ok((id, addr))
    }

    fn parse_ch(line: &str) -> Result<u8, &str> {
        match line.parse::<u8>() {
            Ok(ch) => Ok(ch),
            Err(e) => Err(line),
        }
    }

    fn parse_auto_off(line: &str) -> Result<(u8, u64), &str> {
        let v: Vec<&str> = line.split(' ').collect();
        if v.len() != 2 {
            return Err(line);
        }
        match (v[0].parse::<u8>(), v[1].parse::<u64>()) {
            (Ok(a), Ok(b)) => Ok((a, b)),
            (_, _) => Err(line),
        }
    }

    fn parse_ok(line: &str) -> Result<AcknowledgeType, &str> {
        match line {
            "" => Ok(AcknowledgeType::Alive),
            "ESP_RESTART" => Ok(AcknowledgeType::RestartAP),
            "WIFI_ON" => Ok(AcknowledgeType::StartWifi),
            "QUIT_CONFIG" => Ok(AcknowledgeType::QuitConfig),
            _ => Err(line)
        }
    }

    fn parse_datamode(line: &str) -> Result<u8, &str> {
        match line.parse::<u8>() {
            Ok(dm) => Ok(dm),
            Err(e) => Err(line),
        }
    }

    trait Parseable<T> {
        fn parse_data<'a>(value: T) -> Result<Datagram, &'a str>;
    }
    
    impl Parseable<[u8; 10]> for Datagram {
        fn parse_data<'a>(value: [u8; 10]) -> Result<Datagram, &'a str> {
            Err("Not implemented")
        }
    }

    impl Parseable<[u8; 12]> for Datagram {
        fn parse_data<'a>(value: [u8; 12]) -> Result<Datagram, &'a str> {
            Err("Not implemented")
        }
    }

    impl Parseable<[u8; 18]> for Datagram {
        fn parse_data<'a>(value: [u8; 18]) -> Result<Datagram, &'a str> {
            Err("Not implemented")
        }
    }

    impl Parseable<[u8; 20]> for Datagram {
        fn parse_data<'a>(value: [u8; 20]) -> Result<Datagram, &'a str> {
            let id = value[0];
            let battery_voltage = value[1];
            let q1: [u8; 8] = value[2..10].try_into().unwrap();
            let q2: [u8; 8] = value[10..18].try_into().unwrap();
            let quaternions = [
                Some(Quaternion::from(q1)),
                Some(Quaternion::from(q2)),
                None,
                None,
            ];
            let ahrs_enable = value[18];
            let magnetic_power = value[19];

            Ok(Datagram {
                id: id,
                battery_voltage: battery_voltage,
                quaternions: quaternions,
                ahrs_enable: ahrs_enable,
                magnetic_power: magnetic_power,
            })
        }
    }

    impl Parseable<[u8; 34]> for Datagram {
        fn parse_data<'a>(value: [u8; 34]) -> Result<Datagram, &'a str> {
            Err("Not implemented")
        }
    }

    impl Parseable<[u8; 36]> for Datagram {
        fn parse_data<'a>(value: [u8; 36]) -> Result<Datagram, &'a str> {
            Err("Not implemented")
        }
    }

    impl From<Vec<u8>> for Response {
        fn from(buffer: Vec<u8>) -> Self {
            let line = String::from_utf8_lossy(&buffer);
            let line = &line.trim();
            match parse(&line) {
                Ok(res) => res,
                Err(_) => Response::Error,
            }
            
        }
    }
}