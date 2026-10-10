use super::super::*;

/// max samples kept on the power-draw history (3 s cadence ≈ 3 min window)
pub(crate) const PW_HISTORY: usize = 60;

impl Shell {
    /// Sample battery + GPU watts and append to the history. Throttled to the
    /// 3 s services beat; returns true when a sample was pushed.
    pub(crate) fn poll_powerdraw(&mut self) -> bool {
        if let Some(next) = self.pw_next {
            if Instant::now() < next {
                return false;
            }
        }
        self.pw_next = Some(Instant::now() + std::time::Duration::from_secs(3));
        let sample = (self.battery_watts, gpu_watts());
        self.pw_history.push_back(sample);
        while self.pw_history.len() > PW_HISTORY {
            self.pw_history.pop_front();
        }
        true
    }
}

/// GPU power draw in watts from hwmon (`power*_average`, µW) — amdgpu /
/// radeon only; 0.0 when the chip doesn't expose a power sensor.
pub(crate) fn gpu_watts() -> f32 {
    let Ok(entries) = std::fs::read_dir("/sys/class/hwmon") else {
        return 0.0;
    };
    for e in entries.flatten() {
        let base = e.path();
        let Ok(name) = std::fs::read_to_string(base.join("name")) else {
            continue;
        };
        if !(name.trim().eq_ignore_ascii_case("amdgpu")
            || name.trim().eq_ignore_ascii_case("radeon"))
        {
            continue;
        }
        let Ok(power) = std::fs::read_dir(&base) else {
            continue;
        };
        for fe in power.flatten() {
            let f = fe.file_name().to_string_lossy().into_owned();
            if f.starts_with("power") && f.ends_with("_average") {
                if let Ok(raw) = std::fs::read_to_string(fe.path()) {
                    if let Ok(w) = raw.trim().parse::<f32>() {
                        return w / 1e6;
                    }
                }
            }
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        let cfg: crate::config::Config =
            toml::from_str(crate::config::DEFAULT_SHELL_TOML).expect("default config parses");
        Shell::new(cfg)
    }

    #[test]
    fn history_capped_and_throttled() {
        let mut s = shell();
        for _ in 0..(PW_HISTORY + 10) {
            s.pw_history.push_back((1.0, 2.0));
            while s.pw_history.len() > PW_HISTORY {
                s.pw_history.pop_front();
            }
        }
        assert_eq!(s.pw_history.len(), PW_HISTORY);
        // freshly sampled → throttled
        assert!(s.poll_powerdraw(), "first sample pushed");
        assert!(!s.poll_powerdraw(), "no second sample within 3 s window");
    }
}