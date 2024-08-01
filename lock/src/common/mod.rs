use regex::Regex;
use serde::{Deserialize, Serialize};

// lock.json format v1
// array of indices (to flip)

// lock.json format v2

#[derive(Debug, Serialize, Deserialize)]
pub struct LockValidationRecord {
    index: i32,
    locked: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockFormatV2 {
    version: i32,
    flip_indices: Vec<i32>,
    lock_indices: Vec<i32>,
    unlock_indices: Vec<i32>,
    validation: Vec<LockValidationRecord>,
}

// internal format

#[derive(PartialEq, Debug)]
pub enum LockActionType {
    ValidateLocked,
    ValidateUnlocked,
    Lock,
    Unlock,
    Flip,
}

#[derive(Debug)]
pub struct LockAction {
    pub target: i32,
    pub type_: LockActionType,
}

impl LockAction {
    pub fn from_v1(flip_indices: &Vec<i32>) -> Vec<LockAction> {
        let mut actions: Vec<LockAction> = flip_indices
            .iter()
            .map(|x| LockAction {
                target: *x,
                type_: LockActionType::Flip,
            })
            .collect();

        actions.sort_by_key(|x| x.target);

        return actions;
    }

    pub fn from_v2(data: &LockFormatV2) -> Vec<LockAction> {
        let mut actions: Vec<LockAction> = Vec::new();

        data.flip_indices.iter().for_each(|x| {
            actions.push(LockAction {
                target: *x,
                type_: LockActionType::Flip,
            })
        });

        data.lock_indices.iter().for_each(|x| {
            actions.push(LockAction {
                target: *x,
                type_: LockActionType::Lock,
            })
        });

        data.unlock_indices.iter().for_each(|x| {
            actions.push(LockAction {
                target: *x,
                type_: LockActionType::Unlock,
            })
        });

        data.validation.iter().for_each(|x| {
            actions.push(LockAction {
                target: x.index,
                type_: if x.locked {
                    LockActionType::ValidateLocked
                } else {
                    LockActionType::ValidateUnlocked
                },
            })
        });

        actions.sort_by_key(|x| x.target);

        return actions;
    }

    pub fn validate(actions: &Vec<LockAction>) -> Result<(), String> {
        let mut a_target = i32::MAX;
        let mut n_validation = 0;
        let mut n_click = 0;

        for a in actions.iter() {
            if a.target != a_target {
                a_target = a.target;
                n_validation = 0;
                n_click = 0;
            }
            if a.type_ == LockActionType::ValidateLocked
                || a.type_ == LockActionType::ValidateUnlocked
            {
                n_validation += 1;
            } else {
                n_click += 1;
            }

            if n_validation > 1 || n_click > 1 {
                return Err(format!("Lock action conficts for index {}", a_target));
            }
        }

        Ok(())
    }

    pub fn from_lock_json(json_str: &str) -> Result<Vec<LockAction>, String> {
        let mut actions: Vec<LockAction> = Vec::new();
        // v1
        let re_v1 = Regex::new(r"^\s*\[[\d,\s]*\]\s*$").unwrap();
        if re_v1.is_match(json_str) {
            let flip_indices: Vec<i32> = serde_json::from_str(json_str).unwrap();
            actions = Self::from_v1(&flip_indices);
            Self::validate(&actions).unwrap();
            return Ok(actions);
        }

        // v2
        let re_v2 = Regex::new(r#""version"\s*:\s*2"#).unwrap();
        if re_v2.is_match(json_str) {
            let data: LockFormatV2 = serde_json::from_str(json_str).unwrap();
            actions = Self::from_v2(&data);
            Self::validate(&actions).unwrap();
            return Ok(actions);
        };

        return Err(format!("Unknown lock.json version"));
    }
}
