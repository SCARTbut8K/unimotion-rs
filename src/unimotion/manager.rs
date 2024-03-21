use self::device::UniSensorDevice;

use super::*;
use macaddr::MacAddr6;
use device::{SensorInfo, Response, Datagram, AcknowledgeType};

use std::io::BufRead;
use std::option::Option::Some;
use std::sync::{Mutex, Once};
use std::time::Duration;
use std::thread::JoinHandle;

use serialport::SerialPort;

pub const MAX_UNISENSOR_COUNT: usize = 24;

// #[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
// #[derive(Copy, Clone, Hash, PartialEq, Eq, Debug)]
pub enum Command {
    RestartAP,
    Alive,
    ListSensor,
    StartWifi,
    QuitConfig,
    RequestSensorInfo(u8),
    AliveNoResponse,
    EnableAhrs(u8),
    DisableAhrs(u8),
    Set60FPS(u8),
    Set60FPSLowPower(u8),
    Set70FPS(u8),
    Set144FPS(u8),
    PowerOffSensor(u8),
    RestartSensor(u8),
    StartMagneticCalibration(u8),
    StopMagneticCalibration(u8),
    SetMagneticThreshold(u8, u8, u8),
    SensorConfig(u8),
    Config(u8),
    InitializeCalibration(u8),
    Restart(u8),
    SavePairing,
}

impl Command {
    pub fn as_str(&self) -> String {
        match self {
            Command::RestartAP => String::from("_aprestart"),
            Command::Alive => String::from("_alive"),
            Command::ListSensor => String::from("_sensorlist"),
            Command::StartWifi => String::from("_wifistart"),
            Command::QuitConfig => String::from("_quitconfig"),
            Command::RequestSensorInfo(id) => format!("__sensinfo id:{id}:b"),
            Command::AliveNoResponse => String::from("_alive_nores"),
            Command::EnableAhrs(id) => format!("_setahrsmode id:{id}:b 0"),
            Command::DisableAhrs(id) => format!("_setahrsmode id:{id}:b 1"),
            Command::Set60FPS(id) => format!("_setmode id:{id}:b 3 2 0 4"),
            Command::Set60FPSLowPower(id) => format!("_setmode id:{id}:b 4 2 11 4"),
            Command::Set70FPS(id) => format!("_setmode id:{id}:b 0 9 19 0"),
            Command::Set144FPS(id) => format!("_setmode id:{id}:b 2 4 30 4"),
            Command::PowerOffSensor(id) => format!("_sensoff id:{id}:b"),
            Command::RestartSensor(id) => format!("_restart id:{id}:b"),
            Command::StartMagneticCalibration(id) => format!("_start_mag_calib id:{id}:b"),
            Command::StopMagneticCalibration(id) => format!("_stop_mag_calib id:{id}:b"),
            Command::SetMagneticThreshold(id, min, max) => format!("_set_mag_th id:{id}:b {min} {max}"),
            Command::SensorConfig(id) => format!("_sensconf id:{id}:b"),
            Command::Config(id) => format!("_config"),
            Command::InitializeCalibration(id) => format!("_initcalibration id:{id}"),
            Command::Restart(id) => format!("_restart id:{id}"),
            Command::SavePairing => String::from("_savepairing"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct UnimotionSerialNumber(pub String);

pub struct Channels {
    // Multi-producer multi-consumer channels for message passing.
    sensor_info_rx: crossbeam_channel::Receiver<(u8, SensorInfo)>,
    device_rx: crossbeam_channel::Receiver<(u8, MacAddr6)>,
    channel_rx: crossbeam_channel::Receiver<u8>,
    auto_off_rx: crossbeam_channel::Receiver<(u8, u64)>,
    acknowledge_rx: crossbeam_channel::Receiver<AcknowledgeType>,
    datamode_rx: crossbeam_channel::Receiver<u8>,
    data_rx: crossbeam_channel::Receiver<Datagram>,
    error_rx: crossbeam_channel::Receiver<()>,
}

pub struct UnimotionManager {
    ingress_thread: Option<JoinHandle<()>>,
    port: Box<dyn SerialPort>,
    sensors: [UniSensorDevice; MAX_UNISENSOR_COUNT],
    // Multi-producer multi-consumer channels for message passing.
    channels: Channels,
}

impl UnimotionManager {
    /// Get `UnimotionManager` instance.
    pub fn get_instance() -> Arc<Mutex<Self>> {
        static mut SINGLETON: Option<Arc<Mutex<UnimotionManager>>> = None;
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                let instance = UnimotionManager::new().unwrap();

                SINGLETON = Some(instance);
            });

            match SINGLETON.clone() {
                Some(manager) => manager,
                None => unreachable!(),
            }
        }
    }

    /// Constructor
    fn new() -> UnimotionResult<Arc<Mutex<Self>>> {
        let (sensor_info_tx, sensor_info_rx) = crossbeam_channel::unbounded();
        let (device_tx, device_rx) = crossbeam_channel::unbounded();
        let (channel_tx, channel_rx) = crossbeam_channel::unbounded();
        let (auto_off_tx, auto_off_rx) = crossbeam_channel::unbounded();
        let (acknowledge_tx, acknowledge_rx) = crossbeam_channel::unbounded();
        let (datamode_tx, datamode_rx) = crossbeam_channel::unbounded();
        let (data_tx, data_rx) = crossbeam_channel::unbounded();
        let (error_tx, error_rx) = crossbeam_channel::unbounded();

        let output = serialport::new("/dev/ttyUSB0", 230_400)
            .timeout(Duration::from_millis(1000))
            .open()?;
        let input = output.try_clone()?;

        let manager = {
            let manager = UnimotionManager {
                ingress_thread: None,
                port: output,
                sensors: [UniSensorDevice::empty(); MAX_UNISENSOR_COUNT],
                // Consumer channels.
                channels: Channels {
                    sensor_info_rx,
                    device_rx,
                    channel_rx,
                    auto_off_rx,
                    acknowledge_rx,
                    datamode_rx,
                    data_rx,
                    error_rx,
                }
            };
            Arc::new(Mutex::new(manager))
        };

        let ingress_thread = {
            let mut reader = std::io::BufReader::new(input);
            
            std::thread::spawn(move || {
                let mut buffer = Vec::new();
                loop {
                    match reader.read_until(b'\n', &mut buffer) {
                        Ok(n) => {
                            // Print raw bytes
                            println!("Read {} bytes: {:?}", buffer.len(), &buffer[0..n]);
                            print!("ASCII: {}", String::from_utf8_lossy(&buffer[0..n]));
                            
                            let sent = match Response::from(buffer.clone()) {
                                Response::SensorInfo(id, info) => { 
                                    sensor_info_tx.send((id, info)).is_ok()
                                },
                                Response::Device(id, addr) => { 
                                    device_tx.send((id, addr)).is_ok()
                                },
                                Response::Channel(channel) => { 
                                    channel_tx.send(channel).is_ok()
                                },
                                Response::AutoOff(enable, duration) => { 
                                    auto_off_tx.send((enable, duration)).is_ok()
                                },
                                Response::Acknowledge(ack) => { 
                                    acknowledge_tx.send(ack).is_ok()
                                },
                                Response::Datamode(dm) => { 
                                    datamode_tx.send(dm).is_ok()
                                },
                                Response::Data(data) => { 
                                    data_tx.send(data).is_ok()
                                },
                                Response::Error => { 
                                    error_tx.send(()).is_ok()
                                },
                            };

                            if sent != true { return }
                            buffer.clear();
        
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            eprintln!("Error reading from serial port: {:?}", e);
                            break; // Break the loop on error (you can handle it differently based on your requirements)
                        }
                    }
                }
            })
        };

        {
            let mut manager = match manager.lock() {
                Ok(m) => m,
                Err(m) => m.into_inner(),
            };
            manager.ingress_thread = Some(ingress_thread);
            // UniStation initialization routine
            manager.begin()?
        }
        Ok(manager)
    }

    /// Initialize the UniStation
    pub fn begin(&mut self) -> UnimotionResult<()> {
        use PlaceholderError::*;
        let output = &mut self.port;

        println!("AP Restart");
        writeln!(output, "{}", Command::RestartAP.as_str())?;
        match Self::get_acknowledge_timeout(&mut  self.channels, Duration::from_millis(500)) {
            Ok(AcknowledgeType::RestartAP) => (),
            Ok(ack) => 
                return Err(UnimotionError::from(UnexpectedAckError(AcknowledgeType::RestartAP, ack))),
            Err(_) => 
                return Err(UnimotionError::CrossbeamChannelError),
        };

        match Self::get_channel_timeout(&mut  self.channels, Duration::from_millis(500)) {
            Ok(ch) => (), Err(_) => return Err(UnimotionError::CrossbeamChannelError),
        };

        match Self::get_datamode_timeout(&mut  self.channels, Duration::from_millis(500)) {
            Ok(dm) => (), Err(_) => return Err(UnimotionError::CrossbeamChannelError),
        };

        match Self::get_auto_off_timeout(&mut  self.channels, Duration::from_millis(500)) {
            Ok((en, duration)) => (), Err(_) => return Err(UnimotionError::CrossbeamChannelError),
        };

        match Self::get_devices_timeout(&mut  self.channels, Duration::from_millis(500)) {
            Ok(devices) => {
                for (id, addr) in devices {
                    if addr.is_nil() == false {
                        self.sensors[id as usize] = UniSensorDevice {
                            id: id,
                            mac_addr: addr,
                            sensor_info: None
                        }
                    }
                }
            }, 
            Err(_) => return Err(UnimotionError::CrossbeamChannelError),
        };

        println!("Alive?");
        writeln!(output, "{}", Command::Alive.as_str())?;
        match Self::get_acknowledge_timeout(&mut self.channels, Duration::from_millis(500)) {
            Ok(AcknowledgeType::Alive) => (),
            Ok(ack) => 
                return Err(UnimotionError::from(UnexpectedAckError(AcknowledgeType::Alive, ack))),
            Err(_) => 
                return Err(UnimotionError::CrossbeamChannelError),
        };

        println!("Start wifi");
        writeln!(output, "{}", Command::StartWifi.as_str())?;
        match Self::get_acknowledge_timeout(&mut self.channels, Duration::from_millis(500)) {
            Ok(AcknowledgeType::StartWifi) => (),
            Ok(ack) => 
                return Err(UnimotionError::from(UnexpectedAckError(AcknowledgeType::StartWifi, ack))),
            Err(_) => 
                return Err(UnimotionError::CrossbeamChannelError),
        };

        println!("Quit config");
        writeln!(output, "{}", Command::QuitConfig.as_str())?;
        match Self::get_acknowledge_timeout(&mut self.channels, Duration::from_millis(500)) {
            Ok(AcknowledgeType::QuitConfig) => (),
            Ok(ack) => 
                return Err(UnimotionError::from(UnexpectedAckError(AcknowledgeType::QuitConfig, ack))),
            Err(_) => 
                return Err(UnimotionError::CrossbeamChannelError),
        };

        Ok(())
    }

    // pub fn get_sensor_info(chls: &mut Channels) -> Result<(u8, SensorInfo), crossbeam_channel::RecvError> {
    // }

    // pub fn get_sensor_info_timeout(chls: &mut Channels, timeout: Duration) -> Result<(u8, SensorInfo), crossbeam_channel::RecvError> {
    // }

    pub fn get_devices(chls: &mut Channels) -> Result<[(u8, MacAddr6); MAX_UNISENSOR_COUNT], crossbeam_channel::RecvError> {
        let mut counter = 0;
        let mut res = [(255, MacAddr6::nil()); MAX_UNISENSOR_COUNT];
        loop {
            let _ = match chls.device_rx.recv() {
                Ok((id, addr)) => res[id as usize] = (id, addr),
                Err(e) => break Err(e),
            };
            counter += 1;
            if counter >= MAX_UNISENSOR_COUNT { break Ok(res); }
        }
    }

    pub fn get_devices_timeout(chls: &mut Channels, timeout: Duration) -> Result<[(u8, MacAddr6); MAX_UNISENSOR_COUNT], crossbeam_channel::RecvTimeoutError> {
        let mut counter = 0;
        let mut res = [(255, MacAddr6::nil()); MAX_UNISENSOR_COUNT];
        loop {
            let _ = match chls.device_rx.recv_timeout(timeout) {
                Ok((id, addr)) => res[id as usize] = (id, addr),
                Err(e) => break Err(e),
            };
            counter += 1;
            if counter >= MAX_UNISENSOR_COUNT { break Ok(res); }
        }
    }

    pub fn get_channel(chls: &mut Channels) -> Result<u8, crossbeam_channel::RecvError> {
        chls.channel_rx.recv()
    }

    pub fn get_channel_timeout(chls: &mut Channels, timeout: Duration) -> Result<u8, crossbeam_channel::RecvTimeoutError> {
        chls.channel_rx.recv_timeout(timeout)
    }

    pub fn get_auto_off(chls: &mut Channels) -> Result<(u8, u64), crossbeam_channel::RecvError> {
        chls.auto_off_rx.recv()
    }

    pub fn get_auto_off_timeout(chls: &mut Channels, timeout: Duration) -> Result<(u8, u64), crossbeam_channel::RecvTimeoutError> {
        chls.auto_off_rx.recv_timeout(timeout)
    }

    pub fn get_acknowledge(chls: &mut Channels) -> Result<AcknowledgeType, crossbeam_channel::RecvError> {
        chls.acknowledge_rx.recv()
    }

    pub fn get_acknowledge_timeout(chls: &mut Channels, timeout: Duration) -> Result<AcknowledgeType, crossbeam_channel::RecvTimeoutError> {
        chls.acknowledge_rx.recv_timeout(timeout)
    }

    pub fn get_datamode(chls: &mut Channels) -> Result<u8, crossbeam_channel::RecvError> {
        chls.datamode_rx.recv()
    }

    pub fn get_datamode_timeout(chls: &mut Channels, timeout: Duration) -> Result<u8, crossbeam_channel::RecvTimeoutError> {
        chls.datamode_rx.recv_timeout(timeout)
    }

    pub fn get_data(chls: &mut Channels) -> Result<Datagram, crossbeam_channel::RecvError> {
        chls.data_rx.recv()
    }

    pub fn get_data_timeout(chls: &mut Channels, timeout: Duration) -> Result<Datagram, crossbeam_channel::RecvTimeoutError> {
        chls.data_rx.recv_timeout(timeout)
    }
    
    pub fn flush(chls: &mut Channels) {
        while let Ok(value) = chls.sensor_info_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.device_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.channel_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.auto_off_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.acknowledge_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.datamode_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.data_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
        while let Ok(value) = chls.error_rx.try_recv() {
            println!("Discarding {:?}", value);
        }
    }

    pub fn send_command(&mut self, cmd: Command) -> UnimotionResult<()> {
        let output = &mut self.port;
        match writeln!(output, "{}", cmd.as_str()) {
            Err(e) => Err(UnimotionError::from(e)),
            Ok(()) => Ok(()),
        }
    }

    pub fn sensors(&self) -> Vec<UniSensorDevice> {
        let mut v = Vec::new();
        for sensor in self.sensors {
            if sensor.mac_addr.is_nil() == false {
                v.push(sensor)
            }
        }
        return v
    }

    pub fn update(&mut self) -> (UniSensorDevice, Datagram) {
        let data = Self::get_data(&mut self.channels).unwrap();
        (self.sensors[data.id as usize], data)
    }
}
