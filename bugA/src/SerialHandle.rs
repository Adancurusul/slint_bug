

pub mod SerialHandle {
    pub use crate::CONN as CONN;
    use std::collections::HashMap;
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value;
    pub use serialport::{self, SerialPort};
    pub use std::io;
    pub use std::sync::{Arc, Mutex};
    pub use std::thread;
    pub use std::time::{Duration, Instant};
    //pub mod SQLHandle;
    pub use crate::SQLHandle::SQLHandle::*;
    //pub mod TimeConvert;
    //pub use TimeConvert::TimeConvert::*;
    pub use crate::TimeConvert::TimeConvert::*;
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct DeviceData {
        pub nm: String,
        pub t: f32,
        pub c: i64,
        pub record_time: Option<i64>,
    }
    impl Clone for DeviceData {
        fn clone(&self) -> Self {
            DeviceData {
                nm: self.nm.clone(),
                t: self.t,
                c: self.c,
                record_time: self.record_time.clone(),
            }
        }
    }
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct DeviceListData {
        pub deviceName: String,
        pub exp_time: Option<i64>,
        pub data_volume: Option<i64>,
        pub total_count: Option<i32>,
        pub is_deleted: Option<i64>,
        pub is_show: Option<i64>,
        pub exp_id: Option<i32>,
    }
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct CachedDataStruct {
        pub id: Option<i64>,
        pub deviceName: String,
        pub exp_time: Option<i64>,
        pub data_volume: Option<i64>,
    }
    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    pub struct HistoryStruct {
        pub id: Option<i64>,
        pub exp_time: Option<i64>,
        pub data_volume: Option<i64>,
    }
    pub struct TableShowIndex {
        pub firstInd: Option<i64>,
        pub lastInd: Option<i64>,
    }
    pub struct RuntimeData {
        pub deviceList: HashMap<String, DeviceListData>,  //实时
        pub deviceData: HashMap<String, Vec<DeviceData>>, //实时
        pub currentHisList: Vec<HistoryStruct>,           //history 数据索引列表//参数为时间戳
        pub currentHisData: Vec<DeviceData>,              //当前选中的history 数据
        pub cachedData: Vec<CachedDataStruct>,
        pub currentCachedDataHis: Vec<HistoryStruct>,
        pub cachedDetetedData: Vec<DeviceData>,
        pub currentDeviceName: String,
        pub newDataFlag : bool,
        pub dataTableIndex : TableShowIndex,
        pub historyDataTableIndex : TableShowIndex,
        //pub currentDeviceDataCount: i64,
        //currentData: Vec<DeviceData>,//当前选中设备的数据//可以通过deviceData：currentDeviceName 访问
    }
    impl RuntimeData {
        pub fn new() -> Self {
            RuntimeData {
                deviceList: HashMap::new(),
                deviceData: HashMap::new(),
                currentHisList: Vec::new(),
                currentHisData: Vec::new(),
                cachedData: Vec::new(),
                currentCachedDataHis: Vec::new(),
                cachedDetetedData: Vec::new(),
                currentDeviceName: String::new(),
                newDataFlag : false,
                dataTableIndex: TableShowIndex {
                    firstInd: Some(0 as i64),
                    lastInd: Some(30),
                },
                historyDataTableIndex: TableShowIndex {
                    firstInd: None,
                    lastInd: None,
                },
                //currentDeviceDataCount : 0 ,
            }
        }
    }

    pub fn send_request(port: &mut Box<dyn SerialPort>) -> Result<String, String> {
        port.write_all(b"{\"tp\":1}\r\n")
            .map_err(|err| err.to_string())?;
        println!("Sent request");

        let mut response = String::new();
        let start_time = Instant::now();

        while start_time.elapsed() < Duration::from_secs(1) {
            let mut buf = [0u8; 256];
            if let Ok(len) = port.read(&mut buf) {
                let data = String::from_utf8_lossy(&buf[..len]);
                response.push_str(&data);
                println!("Received data: {}", data);
                if response.contains('\n') {
                    response = response.split('\n').collect::<Vec<&str>>()[0].to_string();
                    if let Ok(response_bytes) = serde_json::from_str::<Value>(&response) {
                        if let Some(tp) = response_bytes.get("tp").and_then(|tp| tp.as_u64()) {
                            if tp == 1 || tp == 6 {
                                println!("Received response: {}", response);
                                return Ok(response);
                            }
                        }
                    }
                    response.clear();
                }
            }
        }

        println!("Communication failed");
        Err("Communication failed".to_string())
    }

    // 将设备名称 `nm` 对应的设备在 `device_list` 中查找，如果存在则将其 `is_deleted` 字段设为 0，如果不存在则插入新的设备信息
    pub fn insert_into_device_list(device_list: &mut HashMap<String, DeviceListData>, nm: &str) {
        if let Some(device) = device_list.get_mut(nm) {
            let dataNow = device.data_volume.unwrap() + 1;
            device.data_volume = Some(dataNow);
            device.is_deleted = Some(0);
            MAIN_modify_data_volume_by_id(&CONN.lock().unwrap(), device.exp_id.unwrap(), dataNow)
                .unwrap();
        } else {
            // 如果设备不存在，则插入新的设备信息
            let timeNow = get_timestamp();

            // 在数据库中插入新的数据
            let main_data = MainExpData {
                id: 1,
                device_id: nm.to_string(),
                exp_time: Some(timeNow),
                data_volume: Some(1),
                is_deleted: Some(0),
            };
            let conn = CONN.lock().unwrap();
            MAIN_sql_insert_main_table(&conn, &main_data).unwrap();

            // 从数据库获取刚插入的数据
            let main_data_list =
                MAIN_get_data_by_device_id_and_exp_time(&conn, nm, timeNow).unwrap();
            println!(
                "MainExpData for deviceID {} and expTime {}: {:?}",
                nm, timeNow, main_data_list
            );

            // 创建新的设备信息并插入到 `device_list` 中
            let new_device = DeviceListData {
                deviceName: nm.to_string(),
                exp_time: Some(timeNow),
                data_volume: Some(1),
                total_count: None,
                is_deleted: Some(0),
                is_show: None,
                exp_id: Some(main_data_list[0].id),
            };
            device_list.insert(nm.to_string(), new_device);
        }
    }

    // 将数据 `data` 插入到设备名称 `nm` 对应的设备数据列表 `device_data` 中
    pub fn insert_into_device_data(runtime_data: &mut RuntimeData, nm: &str, data_struct: DeviceData) {
        let dataNow = &data_struct;
        let idNow = runtime_data.deviceList.get(nm).unwrap().exp_id.unwrap();
        println!("nm: {}", nm.to_string());
        println!(" currentDeviceName : {}", runtime_data.currentDeviceName);
        if nm.to_string() == runtime_data.currentDeviceName {
            runtime_data.newDataFlag = true;
        }
        let temp_data = TempratureData {
            id: 1,
            exp_id: Some(idNow),
            record_time: Some(get_timestamp()),
            data: Some(dataNow.t),
            is_deleted: Some(0),
            count: Some(dataNow.c),
        };
        let conn = CONN.lock().unwrap();
        DATA_sql_insert_data_table(&conn, &temp_data).unwrap();
        if let Some(device_vec) = runtime_data.deviceData.get_mut(nm) {
            device_vec.push(data_struct);
        } else {
            // 如果设备数据列表不存在，则创建新的设备数据列表并插入数据
            let mut new_device_vec = Vec::new();
            new_device_vec.push(data_struct);
            runtime_data
                .deviceData
                .insert(nm.to_string(), new_device_vec);
        }

        println!(
            "dataLength: {}",
            runtime_data.deviceData.get(nm).unwrap().len()
        );
    }

    // 根据设备名称 `del` 在 `device_list` 中查找对应的设备，并将其 `is_deleted` 字段设为 1
    pub fn update_device_list(device_list: &mut HashMap<String, DeviceListData>, del: &str) {
        if let Some(device) = device_list.get_mut(del) {
            device.is_deleted = Some(1);
        }
    }

    pub fn receive_data(
        port: Arc<Mutex<Box<dyn SerialPort>>>,
        shared_data: Arc<Mutex<Vec<DeviceData>>>,
        port_end_signal: Arc<Mutex<bool>>,
        runtime_data: Arc<Mutex<RuntimeData>>,
    ) {
        let mut buf = [0u8; 256];
        let mut response = String::new();
        let mut ifDelete = false;
        loop {
            match port.lock().unwrap().read(&mut buf) {
                Ok(len) => {
                    let mut signal = port_end_signal.lock().unwrap();
                    if *signal == true {
                        *signal = false;
                        println!("stop receive data by signal");
                        break; // Exit the loop if port_end_signal is true
                    }
                    let data = String::from_utf8_lossy(&buf[..len]);
                    response.push_str(&data);
                    println!("Received data: {}", data);
                    if response.contains('\n') {
                        for line in response.split('\n') {
                            if let Ok(json_data) = serde_json::from_str::<Value>(line) {
                                if let (Some(nm), Some(t), Some(c)) = (
                                    json_data.get("nm").and_then(|v| v.as_str()),
                                    json_data.get("t").and_then(|v| v.as_f64()),
                                    json_data.get("c").and_then(|v| v.as_i64()),
                                ) {
                                    let mut runtime_data_temp = runtime_data.lock().unwrap();
                                    let nameNow = nm.to_string();
                                    let data = DeviceData {
                                        nm: nm.to_string(),
                                        t: t as f32,
                                        c,
                                        record_time: Some(get_timestamp()),
                                    };
                                    insert_into_device_list(&mut runtime_data_temp.deviceList, &nm);
                                    insert_into_device_data(&mut runtime_data_temp, &nm, data);
                                } else if let (Some(del), Some(tp)) = (
                                    json_data.get("del").and_then(|v| v.as_str()),
                                    json_data.get("tp").and_then(|v| v.as_i64()),
                                ) {
                                    if tp == 6 {
                                        ifDelete = true;
                                        let mut runtime_data_temp = runtime_data.lock().unwrap();
                                        update_device_list(&mut runtime_data_temp.deviceList, &del);
                                    }
                                }
                            }
                        }
                        response.clear();
                    }
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::TimedOut {
                        let mut signal = port_end_signal.lock().unwrap();
                        if *signal == true {
                            *signal = false;
                            println!("stop receive data");
                            break; // Exit the loop if port_end_signal is true
                        }
                        continue; // Continue the loop if the error is a timeout
                    }
                    println!("Error while reading from port: {}", err);
                    break; // Break the loop for other errors
                }
            }
            if (ifDelete == true) {
                ifDelete = false;
                println!("delete data");
                let err = port.lock().unwrap().write_all(b"{\"tp\":2}\r\n");
            }
        }
        println!("endloop");
    }
   pub fn set_port_end_signal(port_end_signal: Arc<Mutex<bool>>, state: bool) {
        let mut signal = port_end_signal.lock().unwrap(); // 获取 Mutex 的锁
        *signal = state; // 设置共享变量的值
    }
    pub fn auto_connect(
        ports: Vec<serialport::SerialPortInfo>,
        shared_data: Arc<Mutex<Vec<DeviceData>>>,
        port_end_signal: Arc<Mutex<bool>>,
        runtime_data: Arc<Mutex<RuntimeData>>,
    ) -> Result<(String, i64), String> {
        for port in ports {
            let port_name = port.port_name.clone();
            println!("Scanning port: {}", port_name);

            let mut port = match serialport::new(&port_name, 115200)
                .timeout(Duration::from_millis(500))
                .open()
            {
                Ok(port) => port,
                Err(err) => {
                    println!("Failed to open port {}: {}", port_name, err);

                    continue; // Skip to the next port
                }
            };

            match send_request(&mut port) {
                Ok(response) => {
                    if let Ok(response_bytes) = serde_json::from_str::<Value>(&response) {
                        if let Some(tp) = response_bytes.get("tp").and_then(|tp| tp.as_u64()) {
                            if tp == 1 {
                                println!("Successful communication with port: {}", port_name);

                                let shared_port = Arc::new(Mutex::new(port));
                                let port_clone = Arc::clone(&shared_port);

                                thread::spawn(move || {
                                    receive_data(
                                        port_clone,
                                        shared_data,
                                        port_end_signal.clone(),
                                        runtime_data.clone(),
                                    );
                                });
                                return Ok((port_name, get_timestamp()));
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("No response received from port: {}", port_name);
                }
            }
        }

        Err("No device found".to_string())
    }
}
