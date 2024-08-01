use clap::{command, Args, FromArgMatches};
use image::{Rgb, RgbImage};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use std::{cell::RefCell, cmp::min, rc::Rc};
use yas_genshin::scanner::GenshinArtifactScannerConfig;

use common::{LockAction, LockActionType};
use log::{error, info};
use yas::capture::{Capturer, GenericCapturer, WindowsCapturer};
use yas::game_info::{self, GameInfo};
use yas::utils::{color_distance, press_any_key_to_continue};
use yas::window_info::{self, FromWindowInfoRepository, WindowInfoRepository};
use yas::{system_control::SystemControl, utils};
use yas_genshin::application::ArtifactScannerApplication;
use yas_genshin::scanner::artifact_scanner::ArtifactScannerWindowInfo;
use yas_genshin::scanner_controller::repository_layout::{
    GenshinRepositoryScanControllerWindowInfo, GenshinRepositoryScannerLogicConfig,
};
use yas_genshin::{
    scanner::GenshinArtifactScanner,
    scanner_controller::repository_layout::{GenshinRepositoryScanController, ScrollResult},
};

mod common;

pub fn main() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .init();

    let mut command = command!();
    command = <AritfactLockConfig as Args>::augment_args_for_update(command);
    command = <GenshinArtifactScannerConfig as Args>::augment_args_for_update(command);
    command = <GenshinRepositoryScannerLogicConfig as Args>::augment_args_for_update(command);

    let matches = command.get_matches();

    let lock_filename: std::path::PathBuf = Path::new(".").join("lock.json");
    if !lock_filename.exists() {
        error!("no lock.json");
        return;
    }

    let json_str: String = fs::read_to_string(lock_filename).unwrap();
    let actions = LockAction::from_lock_json(&json_str).unwrap();
    let lock = Rc::new(RefCell::new(ArtifactLock::new(&matches)));

    match ArtifactLock::lock(lock.clone(), actions) {
        Err(e) => {
            error!("error: {}", e);
            press_any_key_to_continue();
        },

        Ok(_) => {
            press_any_key_to_continue();
        },
    }
}

pub struct ArtifactLock {
    game_info: GameInfo,
    scanner: GenshinArtifactScanner,
    window_info: ArtifactScannerWindowInfo,
    controller: Rc<RefCell<GenshinRepositoryScanController>>,
    capturer: WindowsCapturer,
    lock_config: AritfactLockConfig,
}

#[derive(Clone, clap::Args)]
pub struct AritfactLockConfig {
    #[arg(
        id = "select-time",
        long = "select-time",
        help = "切换遗器间隔",
        value_name = "SELECT_TIME",
        default_value_t = 0
    )]
    pub select_time: i32,

    #[arg(
        id = "click-time",
        long = "click-time",
        help = "鼠标移动后点击间隔",
        value_name = "CLICK_TIME",
        default_value_t = 0
    )]
    pub click_time: i32,

    #[arg(id = "verbose", long, help = "显示详细信息")]
    pub verbose: bool,
}

impl ArtifactLock {
    pub fn new(arg_matches: &clap::ArgMatches) -> Self {
        let window_info_repository = ArtifactScannerApplication::get_window_info_repository();
        let game_info = ArtifactScannerApplication::get_game_info().unwrap();
        let scanner = GenshinArtifactScanner::from_arg_matches(
            &window_info_repository,
            arg_matches,
            game_info.clone(),
        )
        .unwrap();

        let genshin_window_info =
            GenshinRepositoryScanControllerWindowInfo::from_window_info_repository(
                game_info.window.to_rect_usize().size(),
                game_info.ui,
                game_info.platform,
                &window_info_repository,
            )
            .unwrap();

        let window_info = ArtifactScannerWindowInfo::from_window_info_repository(
            game_info.window.to_rect_usize().size(),
            game_info.ui,
            game_info.platform,
            &window_info_repository,
        )
        .unwrap();

        let controller = Rc::new(RefCell::new(
            GenshinRepositoryScanController::from_arg_matches(
                &window_info_repository,
                arg_matches,
                game_info.clone(),
            )
            .unwrap(),
        ));

        let lock_config = AritfactLockConfig::from_arg_matches(arg_matches).unwrap();

        let capturer = GenericCapturer::new().unwrap();

        ArtifactLock {
            game_info,
            scanner,
            window_info,
            controller,
            capturer,
            lock_config,
        }
    }

    pub fn lock(object: Rc<RefCell<ArtifactLock>>, actions: Vec<LockAction>) -> Result<(), String> {
        if actions.len() == 0 {
            info!("no lock actions");
            return Ok(());
        }
        let mut system_control = SystemControl::new();

        let mut scrolled_rows: i32 = 0;
        let mut start_row: i32 = 0;
        let mut start_action; // 加解锁start位置
        let mut end_action: usize = 0; // 加解锁end位置
        let mut start_art; // 当页第一个圣遗物
        let mut end_art; // 当页最后一个圣遗物

        let total_arts: i32 = object
            .borrow_mut()
            .scanner
            .get_item_count()
            .unwrap()
            .try_into()
            .unwrap();
        let col: i32 = object.borrow_mut().window_info.col;
        let row: i32 = object.borrow_mut().window_info.row;
        let total_rows: i32 = (total_arts + col - 1) / col;

        let window_info = object.borrow_mut().window_info.clone();
        let game_info = object.borrow_mut().game_info.clone();
        let controller = object.borrow_mut().controller.clone();

        let click_time = object.borrow_mut().lock_config.click_time as u32;
        let select_time = object.borrow_mut().lock_config.select_time as u32;

        if actions[actions.len() - 1].target > total_arts.try_into().unwrap() {
            return Err("target out of range".to_string());
        }

        utils::sleep(1000);
        controller.borrow_mut().move_to(0, 0);
        let _ = controller.borrow_mut().sample_initial_color();
        let _ = controller.borrow_mut().wait_until_switched();

        // loop over pages
        'outer: while end_action < actions.len() {
            if utils::is_rmb_down() {
                break 'outer;
            }

            start_action = end_action;
            start_art = col * (scrolled_rows + start_row);
            end_art = min(col * (scrolled_rows + row), total_arts);

            // get actions inside current page
            while end_action < actions.len() && actions[end_action].target < end_art {
                end_action += 1;
            }

            // flip locks
            for i in start_action..end_action {
                let a = &actions[i];
                let p = a.target - start_art;

                if a.type_ == LockActionType::Lock
                    || a.type_ == LockActionType::Unlock
                    || a.type_ == LockActionType::Flip
                {
                    if utils::is_rmb_down() {
                        break 'outer;
                    }
                    let r = p / col + start_row;
                    let c = p % col;

                    controller.borrow_mut().move_to(r as usize, c as usize);
                    utils::sleep(click_time);
                    system_control.mouse_click().unwrap();
                    controller.borrow_mut().wait_until_switched();
                    utils::sleep(select_time);

                    // validate
                    let lock = object.borrow_mut().get_lock(r as f64, c as f64);
                    info!("flip lock of {} at ({}, {}) lock: {}", a.target, r, c, lock);

                    if (a.type_ == LockActionType::ValidateLocked && lock)
                        || (a.type_ == LockActionType::ValidateUnlocked && !lock)
                    {
                        return Err(format!(
                            "Validate error: artifact at {} should be {}",
                            a.target,
                            if lock { "locked" } else { "unlocked" }
                        ));
                    }

                    let left: i32 = game_info.window.left + window_info.art_lock_pos.x as i32;
                    let top: i32 = game_info.window.top + window_info.art_lock_pos.y as i32;

                    system_control.mouse_move_to(left, top);
                    utils::sleep(click_time);
                    system_control.mouse_click();
                    while lock == object.borrow_mut().get_lock(r as f64, c as f64) {
                        continue;
                    }
                    utils::sleep(select_time);
                }
            }

            if utils::is_rmb_down() {
                break 'outer;
            }

            // scroll one page
            if total_rows <= scrolled_rows + row || end_action >= actions.len() {
                break 'outer;
            }

            controller.borrow_mut().move_to(0, 0);
            utils::sleep(select_time + 100);
            let to_scroll_rows = min(total_rows - scrolled_rows - row, row);
            match controller.borrow_mut().scroll_rows(to_scroll_rows) {
                ScrollResult::TimeLimitExceeded => {
                    error!("翻页出现问题");
                    break 'outer;
                },
                ScrollResult::Interrupt => break 'outer,
                _ => (),
            }
            scrolled_rows += to_scroll_rows;
            start_row = row - to_scroll_rows;
        }

        Ok(())
    }

    fn get_lock(&mut self, r: f64, c: f64) -> bool {
        let origin = self.game_info.window;
        let margin = self.window_info.scan_margin_pos;
        let gap = self.window_info.item_gap_size;
        let size = self.window_info.item_size;

        let left = ((origin.left as f64 + margin.x) + (gap.width + size.width) * c) as i32;
        let top = ((origin.top as f64 + margin.y) + (gap.height + size.height) * r) as i32;
        let width = (gap.width + size.width) as i32;
        let height = (gap.height + size.height) as i32;

        let game_image = self
            .capturer
            .capture_rect(yas::positioning::Rect {
                left,
                top,
                width,
                height,
            })
            .unwrap();

        let pos_x = self.window_info.lock_pos.x;
        let pos_y = self.window_info.lock_pos.y;
        let mut locked = false;
        'sq: for dx in -1..1 {
            for dy in -10..10 {
                if pos_y as i32 + dy < 0 || (pos_y as i32 + dy) as u32 >= game_image.height() {
                    continue;
                }

                let color =
                    game_image.get_pixel((pos_x as i32 + dx) as u32, (pos_y as i32 + dy) as u32);

                if color_distance(color, &Rgb([255, 138, 117])) < 30 {
                    if self.lock_config.verbose {
                        info!("locked at ({}, {})", pos_x as i32 + dx, pos_y as i32 + dy);
                    }
                    locked = true;
                    break 'sq;
                }
            }
        }
        if self.lock_config.verbose {
            game_image.save(format!("{}-{}-{}.png", r, c, locked));
        }

        locked
    }
}
