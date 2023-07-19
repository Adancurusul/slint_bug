use plotters::prelude::*;
use rusqlite::{Connection, Result};
use slint::SharedPixelBuffer;
use slint::SharedString;
use slint::{Model, StandardListViewItem, Timer, TimerMode, VecModel};
use std::io;
use std::rc::Rc;
use std::sync::{Arc, Mutex,mpsc};

use std::thread;

mod TimeConvert;
use TimeConvert::TimeConvert::*;
mod SQLHandle;
use SQLHandle::SQLHandle::*;
mod SerialHandle;
use SerialHandle::SerialHandle::*;

slint::include_modules!();
slint::slint! {
    //import {App} from "temp-display.slint";
    //import {PlotterPage} from "temp-display.slint";
}
#[macro_use]
extern crate lazy_static;
//use lazy_static::lazy_static;
lazy_static! {
    pub static ref CONN: Mutex<Connection> = Mutex::new(Connection::open("example.db").unwrap());
}

fn pdf(x: f64, y: f64, a: f64) -> f64 {
    const SDX: f64 = 0.1;
    const SDY: f64 = 0.1;
    let x = x as f64 / 10.0;
    let y = y as f64 / 10.0;
    a * (-x * x / 2.0 / SDX / SDX - y * y / 2.0 / SDY / SDY).exp()
}

fn render_plot(xMiddle: f32, yMiddle: f32) -> slint::Image {
    let mut pixel_buffer = SharedPixelBuffer::new(640, 480);
    let size = (pixel_buffer.width(), pixel_buffer.height());

    let backend = BitMapBackend::with_buffer(pixel_buffer.make_mut_bytes(), size);

    // Plotters requires TrueType fonts from the file system to draw axis text - we skip that for
    // WASM for now.
    #[cfg(target_arch = "wasm32")]
    let backend = wasm_backend::BackendWithoutText { backend };

    let root = backend.into_drawing_area();

    root.fill(&WHITE).expect("error filling drawing area");
    //.caption("Line Plot Demo", ("sans-serif", 40))
    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 40)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(xMiddle - 5.0..xMiddle + 5.5, yMiddle - 5.0..yMiddle + 5.5)
        .expect("error building coordinate system");
    //chart.configure_mesh().draw().unwrap();
    //chart.configure_mesh()
    //    .y_labels(10)
    //    .light_line_style(&TRANSPARENT)
    //    .disable_x_mesh()
    //    .draw()?;
    chart
        .configure_mesh()
        .x_desc("Record Time (s)")
        .y_desc("Temperature (°C)")
        .disable_x_mesh() // 隐藏网格 // 隐藏网格
        .draw()
        .unwrap();
    let DATA = [
        36.1, 37.2, 36.7, 38.6, 36.1, 37.2, 36.7, 38.6, 36.1, 37.2, 36.7, 38.6, 36.1, 37.2, 36.7,
        38.6,
    ];
    chart
        .draw_series(LineSeries::new(
            (0..).zip(DATA.iter()).map(|(idx, price)| {
                let day = (idx / 5) * 7 + idx % 5 + 1;
                let date = idx as f32;
                (date, *price)
            }),
            &BLUE,
        ))
        .unwrap();

    root.present().expect("error presenting");
    drop(chart);
    drop(root);

    slint::Image::from_rgb8(pixel_buffer)
}

fn is_last_ind_valid(global_runtime_data: &RuntimeData) -> bool {
    let current_device_name = &global_runtime_data.currentDeviceName;
    let device_data = global_runtime_data.deviceData.get(current_device_name);

    if let Some(device_data) = device_data {
        let last_ind = global_runtime_data.dataTableIndex.lastInd.unwrap_or(0) as usize;
        last_ind <= device_data.len()
    } else {
        false
    }
}
fn get_show_row_data(global_runtime_data: &Arc<Mutex<RuntimeData>>) -> Rc<Vec<DeviceData>> {
    let global_runtime_data_clone = Arc::clone(&global_runtime_data);
    let show_row_data_clone = Rc::new(Vec::new());

    let runtime_data = global_runtime_data_clone.lock().unwrap();
    let current_device_name = &runtime_data.currentDeviceName;
    let device_data = runtime_data.deviceData.get(current_device_name);

    if let Some(device_data) = device_data {
        let data_table_index = &runtime_data.dataTableIndex;
        let mut first_ind = data_table_index.firstInd.unwrap_or(0) as usize;
        first_ind = first_ind.min(device_data.len());
        let last_ind = data_table_index.lastInd.unwrap_or(device_data.len() as i64) as usize;

        let last_ind = last_ind.min(device_data.len());
        let show_row_data = &device_data[first_ind..last_ind].to_vec();
        
        let temp = Rc::new(show_row_data.clone());
        return  temp;
        //let mut show_row_data_clone_mut = Rc::make_mut(&mut show_row_data_clone);
        //*show_row_data_clone_mut = show_row_data.clone();
    }else{
        show_row_data_clone
    }

    
}

fn init_db() {
    let conn = Connection::open("example.db").unwrap();
    // 创建表
    sql_create_table(&conn).unwrap();
}

fn set_port_end_signal(port_end_signal: Arc<Mutex<bool>>, state: bool) {
    let mut signal = port_end_signal.lock().unwrap(); // 获取 Mutex 的锁
    *signal = state; // 设置共享变量的值
}

pub fn format_time(currentTotalTime: i64) -> String {
    let days = currentTotalTime / (24 * 60 * 60);
    let hours = (currentTotalTime % (24 * 60 * 60)) / (60 * 60);
    let minutes = (currentTotalTime % (60 * 60)) / 60;
    let seconds = currentTotalTime % 60;

    format!("{}D{}H{}M{}S", days, hours, minutes, seconds)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    init_db();

    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();
    let global_runtime_data = Arc::new(Mutex::new(RuntimeData::new()));
    let shared_data: Arc<Mutex<Vec<DeviceData>>> = Arc::new(Mutex::new(Vec::<DeviceData>::new()));
    let port_end_signal = Arc::new(Mutex::new(false));
    let mut start_time = Rc::new(0 as i64);
    let mut plot_data = Vec::<(f32, f32)>::new();
    let main_window = App::new().unwrap();
    let main_window_arc = Arc::new(main_window);
    let main_window_weak = main_window_arc.clone().as_weak();
    let main_window_clone = Arc::clone(&main_window_arc);
    let main_window_clone_global = Arc::clone(&main_window_arc);
    let deviceSuggestTime: i64 = 2160000;
    let current_keys: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));//当前设备列表
    let (sender, receiver) = mpsc::channel();
    let sender = Arc::new(Mutex::new(sender));
    let show_row_data_model: Rc<VecModel<slint::ModelRc<Cell>>> = Rc::new(VecModel::default());
    let show_row_data: Rc<Vec<DeviceData>>;


    main_window_clone_global
        .global::<PlotterPageAdapter>()
        .on_render_plot(render_plot);

    let show_His_model_now: Rc<VecModel<slint::ModelRc<Cell>>> =
        Rc::new(VecModel::default());
    let his_list_model = Rc::new(slint::VecModel::<ItemModel>::default());
    let hisNow = his_list_model.clone();

        let exp_time:String = "2022".to_string();
        let volume = 100;
        let each_cell = Rc::new(slint::VecModel::<Cell>::default());
        each_cell.push(Cell {
            _type: 0,
            value: SharedString::from("1".to_string()),
            editable: false,
            highlighted: false,
        });
        each_cell.push(Cell {
            _type: 0,
            value: SharedString::from(&exp_time),
            editable: false,
            highlighted: false,
        });
        each_cell.push(Cell {
            _type: 0,
            value: SharedString::from(volume.to_string()),
            editable: false,
            highlighted: false,
        });

    show_His_model_now.push(each_cell.into());
    
    main_window_clone_global
        .global::<CT_mainPageAdapter>()
        .set_historyPreviewData(show_His_model_now.clone().into());

    main_window_clone_global
        .global::<CT_mainPageAdapter>()
        .on_onTablePageChanged({
            let mut runtime_data_clone = global_runtime_data.clone();
            let main_window_clone = main_window_clone_global.clone();
            move |firstInd,lastInd|{
                println!("firstInd:{},lastInd:{}", firstInd, lastInd);
                let mut runtime_data_now = runtime_data_clone.lock().unwrap();
                runtime_data_now.dataTableIndex = TableShowIndex {
                    firstInd: Some(firstInd as i64),
                    lastInd: Some(lastInd as i64),
                };
                let total_data = runtime_data_now.deviceData.get(&runtime_data_now.currentDeviceName).unwrap();
                let data_table_index = &runtime_data_now.dataTableIndex;
                let first_ind = data_table_index.firstInd.unwrap_or(0) as usize;
                let last_ind = data_table_index.lastInd.unwrap_or(total_data.len() as i64) as usize;
                let last_ind = last_ind.min(total_data.len());
                let show_row_data = &total_data[first_ind..last_ind].to_vec();           
                //let show_row_data_now: Rc<Vec<DeviceData>> = get_show_row_data(&global_runtime_data_clone);
                let mut firstInd = runtime_data_now.dataTableIndex.firstInd.unwrap_or(0);
                firstInd = firstInd.min(total_data.len() as i64);
                let show_row_data_model_now: Rc<VecModel<slint::ModelRc<Cell>>> = Rc::new(VecModel::default());
                for device_data in show_row_data.iter() {
                    let device_name = &device_data.nm;
                    let temperature = device_data.t;
                    let count = device_data.c;
                    let record_time = device_data.record_time.unwrap_or(-1);
                    let each_cell = Rc::new(slint::VecModel::<Cell>::default());
                    //let mut cell_Small = each_cell.clone();
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(firstInd.to_string()),
                        editable:false,
                        highlighted: false,
                    });
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(timestamp_to_str(record_time)),
                        editable:false,
                        highlighted: false,
                    });
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(temperature.to_string()),
                        editable:false,
                        highlighted: false,
                    });
                    show_row_data_model_now.push(each_cell.into());
                    firstInd+=1;

                }
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_tableData(show_row_data_model_now.clone().into());

            }
        });

    
    let global_runtime_data_clone = Arc::clone(&global_runtime_data);
    let show_row_data_model_clone = Rc::clone(&show_row_data_model);
    
    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        std::time::Duration::from_secs(1),
        move || {
            if let Ok(result) = receiver.try_recv() {
                if result {
                    // 连接成功
                    main_window_clone
                        .global::<LeftSideBarAdapter>()
                        .set_ifConnecting(false);
                    main_window_clone
                        .global::<LeftSideBarAdapter>()
                        .set_connectState(true);
                } else {
                    // 连接失败
                    main_window_clone
                        .global::<LeftSideBarAdapter>()
                        .set_ifConnecting(false);
                    main_window_clone
                        .global::<LeftSideBarAdapter>()
                        .set_connectState(false);
                }
            }
            let mut global_runtime_data_now = global_runtime_data_clone.lock().unwrap();

            if global_runtime_data_now.newDataFlag == true {
                //uploade data to database
                global_runtime_data_now.newDataFlag = false;
                let currentLabDataLen = global_runtime_data_now
                    .deviceData
                    .get(&global_runtime_data_now.currentDeviceName)
                    .unwrap()
                    .len() as i32;
                let currentTotalTime: i64 = global_runtime_data_now
                    .deviceData
                    .get(&global_runtime_data_now.currentDeviceName)
                    .unwrap()
                    .get(currentLabDataLen as usize - 1)
                    .unwrap()
                    .c
                    * 5 as i64;
                let lastTime: i64 = global_runtime_data_now
                    .deviceData
                    .get(&global_runtime_data_now.currentDeviceName)
                    .unwrap()
                    .get(currentLabDataLen as usize - 1)
                    .unwrap()
                    .record_time.unwrap();
                let firstTime: i64 = global_runtime_data_now
                    .deviceData
                    .get(&global_runtime_data_now.currentDeviceName)
                    .unwrap()
                    .get(0)
                    .unwrap()
                    .record_time.unwrap();
                //let currentTotalTimeStr = currentTotalTime / 5;
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_tableDataNum(currentLabDataLen);
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_openTime(SharedString::from(format_time(currentTotalTime)));
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_currentTime(SharedString::from(format_time(lastTime-firstTime)));
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_timeLeft(SharedString::from(format!(
                        "{}D",
                        (deviceSuggestTime - currentTotalTime) / (24 * 60 * 60)
                    )));
                    //: Rc<Vec<DeviceData>>
                
                let total_data = global_runtime_data_now.deviceData.get(&global_runtime_data_now.currentDeviceName).unwrap();
                let data_table_index = &global_runtime_data_now.dataTableIndex;
                let first_ind = data_table_index.firstInd.unwrap_or(0) as usize;
                let last_ind = data_table_index.lastInd.unwrap_or(total_data.len() as i64) as usize;
                let last_ind = last_ind.min(total_data.len());
                let show_row_data = &total_data[first_ind..last_ind].to_vec();           
                //let show_row_data_now: Rc<Vec<DeviceData>> = get_show_row_data(&global_runtime_data_clone);
                let mut firstInd = global_runtime_data_now.dataTableIndex.firstInd.unwrap_or(0);
                let show_row_data_model_now: Rc<VecModel<slint::ModelRc<Cell>>> = Rc::new(VecModel::default());
                for device_data in show_row_data.iter() {
                    let device_name = &device_data.nm;
                    let temperature = device_data.t;
                    let count = device_data.c;
                    let record_time = device_data.record_time.unwrap_or(-1);
                    let each_cell = Rc::new(slint::VecModel::<Cell>::default());
                    //let mut cell_Small = each_cell.clone();
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(firstInd.to_string()),
                        editable:false,
                        highlighted: false,
                    });
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(timestamp_to_str(record_time)),
                        editable:false,
                        highlighted: false,
                    });
                    each_cell.push(Cell {
                        _type: 0,
                        value: SharedString::from(temperature.to_string()),
                        editable:false,
                        highlighted: false,
                    });
                    show_row_data_model_now.push(each_cell.into());
                    firstInd+=1;

                }
                main_window_clone
                    .global::<CT_mainPageAdapter>()
                    .set_tableData(show_row_data_model_now.clone().into());
                let currentTemprature = global_runtime_data_now
                    .deviceData
                    .get(&global_runtime_data_now.currentDeviceName)
                    .unwrap()
                    .get(currentLabDataLen as usize - 1)
                    .unwrap()
                    .t;
                main_window_clone
                    .global::<PlotterPageAdapter>()
                    .set_currentTemprature(SharedString::from(currentTemprature.to_string()));
                println!("Update data");
            }
        },
    );

    
    let timer1 = Timer::default();
    let global_runtime_data_clone = Arc::clone(&global_runtime_data);
    let current_keys_clone = Arc::clone(&current_keys);
    let mainwinClone = Arc::clone(&main_window_arc);
    timer1.start(
        TimerMode::Repeated,
        Duration::from_secs(1),
        move || {

            let new_current_keys: Vec<String> = {
                let runtime_data = global_runtime_data_clone.lock().unwrap();
                runtime_data
                    .deviceList
                    .iter()
                    .filter(|(_, device_data)| device_data.is_deleted == Some(0))
                    .map(|(key, _)| key.clone())
                    .collect()
            };
            

            mainwinClone
                .global::<CT_mainPageAdapter>()
                .set_currentConnect(new_current_keys.contains(&global_runtime_data_clone.lock().unwrap().currentDeviceName));

            {
                let current_keys_guard = current_keys_clone.lock().unwrap();
                let added_keys: Vec<&String> = new_current_keys
                    .iter()
                    .filter(|key| !current_keys_guard.contains(key))
                    .collect();
                let removed_keys: Vec<&String> = current_keys_guard
                    .iter()
                    .filter(|key| !new_current_keys.contains(key))
                    .collect();
                
                if added_keys.len() > 0 || removed_keys.len() > 0 {
                    //println!("Device list changed");
                }

                if added_keys.len() > 0  {
                    let new_current_keys_all: Vec<String> = {//所有都显示，只是用连接标志区分
                        
                        let runtime_data = global_runtime_data_clone.lock().unwrap();
                        runtime_data
                            .deviceList
                            .iter()
                            .map(|(key, _)| key.clone())
                            .collect()
                    };
                    let device_list_model = Rc::new(slint::VecModel::<ItemModel>::default());
                    let deviceNow = device_list_model.clone();
                    for keyNow in  &new_current_keys_all{
                        
                        deviceNow.push(ItemModel {
                            leading_icon: SharedString::from(""),
                            text: SharedString::from(keyNow),
                            trailing_icon: SharedString::from(""),
                            highlighted: false,
                        });
                    }
                    mainwinClone
                        .global::<LeftSideBarAdapter>()
                        .set_deviceCTItems(deviceNow.into());
                    println!("Device list changed");
                }
                if current_keys_guard.len() == 0 {
                    if added_keys.len() > 0 {
                        let mut runtime_data = global_runtime_data_clone.lock().unwrap();
                        runtime_data.currentDeviceName = added_keys.get(0).unwrap().to_string();
                        mainwinClone
                            .global::<LeftSideBarAdapter>()
                            .set_deviceCT(SharedString::from(added_keys.get(0).unwrap().to_string()));
                        mainwinClone
                            .global::<CT_mainPageAdapter>()
                            .set_name(SharedString::from(added_keys.get(0).unwrap().to_string()));
                        
                    }
                }
                for added_key in added_keys {

                    println!("Added: {}", added_key);
                }
                for removed_key in removed_keys {
                    println!("Removed: {}", removed_key);
                }
            }
            *current_keys_clone.lock().unwrap() = new_current_keys;
        },
    );




    //main_window.global::<PlotterPage>().on_render_plot(render_plot);

    main_window_arc.clone().run().unwrap();
    slint::run_event_loop();
}
