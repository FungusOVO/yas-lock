use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::time::SystemTime;

use yas::capture::capture_absolute_image;
use yas::common::utils;
use yas::expo::genmo::GenmoFormat;
use yas::expo::good::GoodFormat;
use yas::expo::mona::MonaFormat;
use yas::info::info;
use yas::scanner::yas_scanner::{YasScanner, YasScannerConfig};

use clap::{App, Arg};
use env_logger::Builder;
use log::{error, info, LevelFilter};

// use enigo::*;

// fn open_local(path: String) -> RawImage {
//     let img = image::open(path).unwrap();
//     let img = grayscale(&img);
//     let raw_img = image_to_raw(img);

//     raw_img
// }

fn get_version() -> String {
    let s = include_str!("../Cargo.toml");
    for line in s.lines() {
        if line.starts_with("version = ") {
            let temp = line.split("\"").collect::<Vec<_>>();
            return String::from(temp[temp.len() - 2]);
        }
    }

    String::from("unknown_version")
}

fn read_lock_file<P: AsRef<Path>>(path: P) -> Result<Vec<u32>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let l: Vec<u32> = serde_json::from_reader(reader)?;

    Ok(l)
}

/*
fn main() {
    let hwnd = match utils::find_window(String::from("原神")) {
        Err(_s) => {
            utils::error_and_quit("未找到原神窗口，请确认原神已经开启");
        }
        Ok(h) => h,
    };

    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
    }
    // utils::sleep(1000);
    unsafe {
        SetForegroundWindow(hwnd);
    }
    let mut enigo = Enigo::new();
    let mut state = 0;
    loop {
        if state == 1 {
            enigo.mouse_click(MouseButton::Left);
            print!(".");
            std::io::stdout().flush().unwrap();
        }
        if utils::is_f12_down() {
            state = 1 - state;
            if state == 0 {
                println!("\n暂停中");
            } else {
                println!("点击中");
            }
        }
        utils::sleep(200);
    }
}
*/

/*
fn gcd(mut a: u32, mut b: u32) -> u32 {
    let mut r;
    while b > 0 {
        r = a % b;
        a = b;
        b = r;
    }
    a
}

fn main() {
    set_dpi_awareness();

    let hwnd = match utils::find_window(String::from("原神")) {
        Err(_s) => {
            utils::error_and_quit("未找到原神窗口，请确认原神已经开启");
        }
        Ok(h) => h,
    };

    unsafe {
        ShowWindow(hwnd, SW_RESTORE);
    }
    unsafe {
        SetForegroundWindow(hwnd);
    }

    utils::sleep(1000);
    let rect = utils::get_client_rect(hwnd).unwrap();

    // rect.scale(1.25);
    // info!("detected left: {}", rect.left);
    // info!("detected top: {}", rect.top);
    // info!("detected width: {}", rect.width);
    // info!("detected height: {}", rect.height);
    let g = gcd(rect.width as u32, rect.height as u32);

    // let date = Local::now();
    capture_absolute_image(&rect)
        .unwrap()
        // .save(format!("{}.png", date.format("%Y_%y_%d_%H_%M_%S")))
        .save(format!(
            "{}x{}_{}x{}.png",
            rect.width as u32 / g,
            rect.height as u32 / g,
            rect.width,
            rect.height
        ))
        .expect("fail to take screenshot");
}
*/

fn start() -> Result<()> {
    if !utils::is_admin() {
        return Err(anyhow!("请以管理员身份运行该程序"));
    }

    let version = get_version();

    let matches = App::new("YAS - 原神圣遗物导出器")
        .version(version.as_str())
        .author("wormtql <584130248@qq.com>")
        .about("Genshin Impact Artifact Exporter")
        .arg(
            Arg::with_name("max-row")
                .long("max-row")
                .takes_value(true)
                .help("最大扫描行数"),
        )
        .arg(
            Arg::with_name("dump")
                .long("dump")
                .takes_value(false)
                .help("输出模型预测结果、二值化图像和灰度图像，debug专用"),
        )
        .arg(
            Arg::with_name("capture-only")
                .long("capture-only")
                .takes_value(false)
                .help("只保存截图，不进行扫描，debug专用"),
        )
        .arg(
            Arg::with_name("mark")
                .long("mark")
                .takes_value(false)
                .help("保存标记后的截图，debug专用"),
        )
        .arg(
            Arg::with_name("min-star")
                .long("min-star")
                .takes_value(true)
                .help("最小星级")
                .possible_values(&["1", "2", "3", "4", "5"]),
        )
        .arg(
            Arg::with_name("min-level")
                .long("min-level")
                .takes_value(true)
                .help("最小等级"),
        )
        .arg(
            Arg::with_name("max-wait-switch-artifact")
                .long("max-wait-switch-artifact")
                .takes_value(true)
                .validator(|t| -> Result<(), String> {
                    if t.parse::<u32>().map_err(|_| String::from("expect int"))? >= 10 {
                        Ok(())
                    } else {
                        Err(String::from("min value: 10"))
                    }
                })
                .help("切换圣遗物最大等待时间(ms)"),
        )
        .arg(
            Arg::with_name("output-dir")
                .long("output-dir")
                .short("o")
                .takes_value(true)
                .help("输出目录")
                .default_value("."),
        )
        .arg(
            Arg::with_name("scroll-stop")
                .long("scroll-stop")
                .takes_value(true)
                .help("翻页时滚轮停顿时间（ms）（翻页不正确可以考虑加大该选项，默认为100）"),
        )
        .arg(
            Arg::with_name("number")
                .long("number")
                .takes_value(true)
                .help("指定圣遗物数量（在自动识别数量不准确时使用）")
                .validator(|n| -> Result<(), String> {
                    let n = n.parse::<u32>().map_err(|_| String::from("expect int"))?;
                    if n >= 1 && n <= 1500 {
                        Ok(())
                    } else {
                        Err(String::from("min value: 1, max value: 1500"))
                    }
                }),
        )
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .help("显示详细信息"),
        )
        .arg(
            Arg::with_name("offset-x")
                .long("offset-x")
                .takes_value(true)
                .help("人为指定横坐标偏移（截图有偏移时可用该选项校正）"),
        )
        .arg(
            Arg::with_name("offset-y")
                .long("offset-y")
                .takes_value(true)
                .help("人为指定纵坐标偏移（截图有偏移时可用该选项校正）"),
        )
        .arg(
            Arg::with_name("speed")
                .long("speed")
                .takes_value(true)
                .help("速度（共1-5档，默认5，如提示大量重复尝试降低速度）")
                .possible_values(&["1", "2", "3", "4", "5"]),
        )
        .arg(
            Arg::with_name("no-check")
                .long("no-check")
                .takes_value(false)
                .help("不检测是否已打开背包等"),
        )
        .arg(
            Arg::with_name("max-wait-scroll")
                .long("max-wait-scroll")
                .takes_value(true)
                .help("翻页的最大等待时间(ms)"),
        )
        .arg(
            Arg::with_name("dxgcap")
                .long("dxgcap")
                .takes_value(false)
                .help("使用dxgcap捕获屏幕"),
        )
        .get_matches();

    let config = YasScannerConfig::from_match(&matches)?;

    let output_dir = Path::new(matches.value_of("output-dir").unwrap_or("."));

    let mut lock_mode = false;
    let mut indices: Vec<u32> = Vec::new();

    let lock_filename = output_dir.join("lock.json");
    if lock_filename.exists() {
        print!("检测到lock文件，输入y开始加解锁，直接回车开始扫描：");
        stdout().flush()?;
        let mut s: String = String::new();
        stdin().read_line(&mut s)?;
        if s.trim() == "y" {
            indices = read_lock_file(lock_filename)?;
            lock_mode = true;
        }
    }

    utils::set_dpi_awareness();

    let hwnd =
        utils::find_window("原神").map_err(|_| anyhow!("未找到原神窗口，请确认原神已经开启"))?;

    utils::show_window_and_set_foreground(hwnd);
    utils::sleep(1000);

    let mut rect = utils::get_client_rect(hwnd)?;

    let offset_x = matches.value_of("offset-x").unwrap_or("0").parse::<i32>()?;
    let offset_y = matches.value_of("offset-y").unwrap_or("0").parse::<i32>()?;

    rect.left += offset_x;
    rect.top += offset_y;

    capture_absolute_image(&rect)?.save("test.png")?;

    info!(
        "left = {}, top = {}, width = {}, height = {}",
        rect.left, rect.top, rect.width, rect.height
    );

    let info: info::ScanInfo;
    if rect.height * 16 == rect.width * 9 {
        info =
            info::ScanInfo::from_16_9(rect.width as u32, rect.height as u32, rect.left, rect.top);
    } else if rect.height * 8 == rect.width * 5 {
        info = info::ScanInfo::from_8_5(rect.width as u32, rect.height as u32, rect.left, rect.top);
    } else if rect.height * 4 == rect.width * 3 {
        info = info::ScanInfo::from_4_3(rect.width as u32, rect.height as u32, rect.left, rect.top);
    } else {
        return Err(anyhow!("不支持的分辨率"));
    }

    let mut scanner = YasScanner::new(info.clone(), config)?;

    // let _ = scanner.test()?;
    if lock_mode {
        scanner.flip_lock(indices)?;
    } else {
        let now = SystemTime::now();
        let results = scanner.scan()?;
        let t = now.elapsed()?.as_secs_f64();
        info!("time: {}s", t);

        // Mona
        let output_filename = output_dir.join("mona.json");
        let mona = MonaFormat::new(&results);
        mona.save(String::from(output_filename.to_str().context("Err")?))?;
        // Genmo
        let output_filename = output_dir.join("genmo.json");
        let genmo = GenmoFormat::new(&results);
        genmo.save(String::from(output_filename.to_str().context("Err")?))?;
        // GOOD
        let output_filename = output_dir.join("good.json");
        let good = GoodFormat::new(&results);
        good.save(String::from(output_filename.to_str().context("Err")?))?;
    }

    Ok(())
}

fn main() {
    Builder::new().filter_level(LevelFilter::Info).init();

    start().unwrap_or_else(|e| error!("{:#}", e));

    info!("按Enter退出");
    let mut s = String::new();
    stdin().read_line(&mut s).expect("Readline error");
}
