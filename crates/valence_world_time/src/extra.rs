use crate::WorldTime;

pub const DAY_LENGTH: u64 = 24000;

/// Notable events of a 24-hour Minecraft day
pub enum DayPhase {
    Day = 0,
    Noon = 6000,
    Sunset = 12000,
    Night = 13000,
    Midnight = 18000,
    Sunrise = 23000,
}

impl From<DayPhase> for u64 {
    fn from(value: DayPhase) -> Self {
        value as Self
    }
}

/// Reference: <https://minecraft.fandom.com/wiki/Daylight_cycle#Moon_phases>
pub enum MoonPhase {
    FullMoon = 0,
    WaningGibbous = 1,
    ThirdQuarter = 2,
    WaningCrescent = 3,
    NewMoon = 4,
    WaxingCrescent = 5,
    FirstQuarter = 6,
    WaxingGibbous = 7,
}

impl From<MoonPhase> for u64 {
    fn from(value: MoonPhase) -> Self {
        value as Self
    }
}

impl WorldTime {
    /// This function ensure that adding time will not resulting in
    /// time_of_day flipping sign.
    pub fn add_time(&mut self, amount: impl Into<i64>) {
        let client_ticking = self.client_time_ticking();
        self.time_of_day = self.time_of_day.abs().wrapping_add(amount.into());
        if self.time_of_day < 0 {
            self.time_of_day = self.time_of_day + i64::MAX + 1;
        }

        self.set_client_time_ticking(client_ticking);
    }

    /// If the client advances world time locally without server updates.
    pub fn client_time_ticking(&self) -> bool {
        self.time_of_day >= 0
    }

    /// Sets if the client advances world time locally without server updates.
    /// Note: If the resulting calculation set time_of_day to 0. This function
    /// will set time -1 if time_of_day is 0 and is time ticking = false to
    /// workaround protocol limitations
    pub fn set_client_time_ticking(&mut self, val: bool) {
        self.time_of_day = if val {
            self.time_of_day.abs()
        } else {
            -self.time_of_day.abs()
        };
    }

    /// Get the time part of `time_of_day`
    pub fn current_day_time(&self) -> u64 {
        self.time_of_day as u64 % DAY_LENGTH
    }

    /// Set the time part of `time_of_day`
    /// Use the [`DayPhase`] enum to easily handle common time
    /// of day events without the need to look up information in the wiki.
    pub fn set_current_day_time(&mut self, time: impl Into<u64>) {
        let client_ticking = self.client_time_ticking();
        self.time_of_day = (self.day() * DAY_LENGTH + time.into() % DAY_LENGTH) as i64;
        self.set_client_time_ticking(client_ticking);
    }

    /// Get the current day part of `time_of_day`
    pub fn day(&self) -> u64 {
        self.time_of_day as u64 / DAY_LENGTH
    }

    /// Set the current day `time_of_day`
    pub fn set_day(&mut self, day: u64) {
        let client_ticking = self.client_time_ticking();
        self.time_of_day = (day * DAY_LENGTH + self.current_day_time()) as i64;
        self.set_client_time_ticking(client_ticking);
    }

    /// Set the time_of_day to the next specified [`DayPhase`]
    pub fn warp_to_next_day_phase(&mut self, phase: DayPhase) {
        let phase_num: u64 = phase.into();
        if self.current_day_time() >= phase_num {
            self.set_day(self.day() + 1);
        }

        self.set_current_day_time(phase_num);
    }

    /// Set the time_of_day to the next specified [`MoonPhase`]
    pub fn wrap_to_next_moon_phase(&mut self, phase: MoonPhase) {
        let phase_no: u64 = phase.into();
        if self.day() % 8 >= phase_no {
            self.set_day(self.day() + 8 - (self.day() % 8))
        }

        self.set_day(self.day() + phase_no - self.day() % 8);
        self.set_current_day_time(DayPhase::Night);
    }
}
