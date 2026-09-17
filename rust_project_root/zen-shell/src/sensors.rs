//! IIO sensor sample for the dashboard Sensors card.
//!
//! Scans `/sys/bus/iio/devices/iio:device*` for an accelerometer, a gyroscope
//! and a magnetometer/compass (whichever the hardware exposes) and reads the
//! latest raw sample + scale. Cheap sysfs reads — safe to call at a sub-Hz
//! cadence from the app poll; never blocks meaningfully.

use std::path::PathBuf;

#[derive(Clone, Debug, Default)]
pub struct Sensors {
    /// true when an accelerometer was found (tilt is then meaningful even at 0)
    pub has_accel: bool,
    /// accelerometer tilt: pitch (deg, up/down around the left-right axis)
    pub pitch: f32,
    /// accelerometer tilt: roll (deg, left/right around the front-back axis)
    pub roll: f32,
    /// compass heading (deg, 0..360, clockwise from north), None → no magneto
    pub heading: Option<f32>,
    /// gyroscope angular velocity x/y/z (deg/s), None → no gyro
    pub gyro: Option<(f32, f32, f32)>,
}

fn device_root() -> PathBuf {
    PathBuf::from("/sys/bus/iio/devices")
}

/// `iio:deviceN` dirs present on the machine.
fn device_dirs() -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(device_root()) else {
        return Vec::new();
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn device_name(dir: &PathBuf) -> String {
    std::fs::read_to_string(dir.join("name"))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn is_accel(name: &str) -> bool {
    name.contains("accel")
}
fn is_gyro(name: &str) -> bool {
    name.contains("gyro")
}
fn is_compass(name: &str) -> bool {
    name.contains("compass") || name.contains("magn") || name.contains("ak8") || name.contains("lsm")
}

/// Read one sysfs attribute as f32 (scaled), 0.0 on any error.
fn read_val(dir: &PathBuf, attr: &str) -> f32 {
    std::fs::read_to_string(dir.join(attr))
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok())
        .unwrap_or(0.0)
}

/// Sample every sensor found. Missing hardware → default values / None.
pub fn sample() -> Sensors {
    let mut s = Sensors::default();
    for dir in device_dirs() {
        let name = device_name(&dir);
        if is_accel(&name) {
            // axis convention varies by chip; we expose pitch/roll around the
            // horizontal plane (gravity), signed like a phone in landscape
            s.has_accel = true;
            let scale = read_val(&dir, "in_accel_scale").max(1e-9);
            let ax = read_val(&dir, "in_accel_x_raw") * scale;
            let ay = read_val(&dir, "in_accel_y_raw") * scale;
            let az = read_val(&dir, "in_accel_z_raw") * scale;
            s.pitch = ax.atan2((ay * ay + az * az).sqrt()).to_degrees();
            s.roll = ay.atan2(az).to_degrees();
        } else if is_gyro(&name) {
            let scale = read_val(&dir, "in_anglvel_scale").max(1e-9);
            let gx = read_val(&dir, "in_anglvel_x_raw") * scale;
            let gy = read_val(&dir, "in_anglvel_y_raw") * scale;
            let gz = read_val(&dir, "in_anglvel_z_raw") * scale;
            // IIO scale is often rad/s — report deg/s
            const RAD2DEG: f32 = 180.0 / std::f32::consts::PI;
            s.gyro = Some((gx * RAD2DEG, gy * RAD2DEG, gz * RAD2DEG));
        } else if is_compass(&name) {
            let mx = read_val(&dir, "in_magn_x_raw");
            let my = read_val(&dir, "in_magn_y_raw");
            if mx != 0.0 || my != 0.0 {
                let mut h = my.atan2(mx).to_degrees();
                if h < 0.0 {
                    h += 360.0;
                }
                s.heading = Some(h);
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_classifiers() {
        assert!(is_accel("accel_3d"));
        assert!(is_gyro("gyro_3d"));
        assert!(is_compass("compass"));
        assert!(is_compass("lsm303dlhc-magn"));
        assert!(!is_accel("gyro_3d"));
    }
}