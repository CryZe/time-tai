#![cfg_attr(not(feature = "std"), no_std)]

use core::ops::{Add, Sub};

use time::{Duration, OffsetDateTime};

// The document starts with 1900 instead of the usual 1970, so that's 70 years
// in seconds.
const LEAP_BASE_OFFSET: i64 = 25567 * 24 * 60 * 60;

// While it may look like there's 10 leap seconds on 1 Jan 1972, that's
// essentially the beginning of the UTC / TAI distinction. So we also apply it
// to all time stamps before that to ensure that these 10 seconds don't show up
// in any durations.
const FIRST_LEAP_SECONDS_DIFF: i64 = 10;

// https://www.ietf.org/timezones/data/leap-seconds.list
const LEAP_SECONDS: &[(i64, i64)] = &[
    // (2272060800 - LEAP_BASE_OFFSET, 10), // 1 Jan 1972
    (2287785600 - LEAP_BASE_OFFSET, 11), // 1 Jul 1972
    (2303683200 - LEAP_BASE_OFFSET, 12), // 1 Jan 1973
    (2335219200 - LEAP_BASE_OFFSET, 13), // 1 Jan 1974
    (2366755200 - LEAP_BASE_OFFSET, 14), // 1 Jan 1975
    (2398291200 - LEAP_BASE_OFFSET, 15), // 1 Jan 1976
    (2429913600 - LEAP_BASE_OFFSET, 16), // 1 Jan 1977
    (2461449600 - LEAP_BASE_OFFSET, 17), // 1 Jan 1978
    (2492985600 - LEAP_BASE_OFFSET, 18), // 1 Jan 1979
    (2524521600 - LEAP_BASE_OFFSET, 19), // 1 Jan 1980
    (2571782400 - LEAP_BASE_OFFSET, 20), // 1 Jul 1981
    (2603318400 - LEAP_BASE_OFFSET, 21), // 1 Jul 1982
    (2634854400 - LEAP_BASE_OFFSET, 22), // 1 Jul 1983
    (2698012800 - LEAP_BASE_OFFSET, 23), // 1 Jul 1985
    (2776982400 - LEAP_BASE_OFFSET, 24), // 1 Jan 1988
    (2840140800 - LEAP_BASE_OFFSET, 25), // 1 Jan 1990
    (2871676800 - LEAP_BASE_OFFSET, 26), // 1 Jan 1991
    (2918937600 - LEAP_BASE_OFFSET, 27), // 1 Jul 1992
    (2950473600 - LEAP_BASE_OFFSET, 28), // 1 Jul 1993
    (2982009600 - LEAP_BASE_OFFSET, 29), // 1 Jul 1994
    (3029443200 - LEAP_BASE_OFFSET, 30), // 1 Jan 1996
    (3076704000 - LEAP_BASE_OFFSET, 31), // 1 Jul 1997
    (3124137600 - LEAP_BASE_OFFSET, 32), // 1 Jan 1999
    (3345062400 - LEAP_BASE_OFFSET, 33), // 1 Jan 2006
    (3439756800 - LEAP_BASE_OFFSET, 34), // 1 Jan 2009
    (3550089600 - LEAP_BASE_OFFSET, 35), // 1 Jul 2012
    (3644697600 - LEAP_BASE_OFFSET, 36), // 1 Jul 2015
    (3692217600 - LEAP_BASE_OFFSET, 37), // 1 Jan 2017
];

const EXPIRES_AT_UTC: i64 = 3896899200 - LEAP_BASE_OFFSET;
const EXPIRES_AT_TAI: i64 = EXPIRES_AT_UTC + LEAP_SECONDS[LEAP_SECONDS.len() - 1].1;

#[derive(Copy, Clone, Debug)]
pub struct TaiDateTime(Duration);

impl TaiDateTime {
    #[cfg(all(
        feature = "std",
        not(any(
            target_os = "android",
            target_os = "emscripten",
            target_os = "fuchsia",
            target_os = "linux"
        ))
    ))]
    pub fn now() -> Self {
        OffsetDateTime::now_utc().into()
    }

    #[cfg(all(
        feature = "std",
        any(
            target_os = "android",
            target_os = "emscripten",
            target_os = "fuchsia",
            target_os = "linux"
        )
    ))]
    pub fn now() -> Self {
        use nix::time::{clock_gettime, ClockId};

        if let Ok(time) = clock_gettime(ClockId::CLOCK_TAI) {
            Self(Duration::new(time.tv_sec() as i64, time.tv_nsec() as i32))
        } else {
            OffsetDateTime::now_utc().into()
        }
    }
}

impl Sub for TaiDateTime {
    type Output = time::Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl Add<Duration> for TaiDateTime {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[cfg(all(feature = "std", windows))]
fn read_additional_leap_seconds() -> Option<Box<[(i64, i64)]>> {
    use std::{
        mem::{self, MaybeUninit},
        ptr,
    };

    use winapi::{
        shared::{
            minwindef::HKEY,
            winerror::{ERROR_MORE_DATA, ERROR_SUCCESS},
        },
        um::{
            winnt::KEY_READ,
            winreg::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, LSTATUS},
        },
    };

    struct RegKey(HKEY);

    impl Drop for RegKey {
        fn drop(&mut self) {
            unsafe {
                RegCloseKey(self.0);
            }
        }
    }

    // https://github.com/microsoft/STL/blob/e28f9561233a58d48d893094ed3a6bc0c5ee6ad9/stl/inc/__msvc_tzdb.hpp#L27
    #[repr(C)]
    struct LeapInfo {
        year: u16,
        month: u16,
        day: u16,
        hour: u16,
        negative: u16,
        _reserved: u16,
    }

    unsafe {
        let mut leap_sec_key = MaybeUninit::uninit();
        let status = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            b"SYSTEM\\CurrentControlSet\\Control\\LeapSecondInformation\0"
                .map(|b| b as _)
                .as_ptr(),
            0,
            KEY_READ,
            leap_sec_key.as_mut_ptr(),
        );
        if status != ERROR_SUCCESS as LSTATUS {
            return None;
        }
        let leap_sec_key = RegKey(leap_sec_key.assume_init());

        let reg_subkey_name = b"LeapSeconds\0".map(|b| b as _);

        let mut byte_size = 0;
        let status = RegQueryValueExW(
            leap_sec_key.0,
            reg_subkey_name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut byte_size,
        );
        if (status != ERROR_SUCCESS as LSTATUS && status != ERROR_MORE_DATA as LSTATUS)
            || byte_size == 0
        {
            return None;
        }

        let count = byte_size as usize / mem::size_of::<LeapInfo>();
        if byte_size as usize % mem::size_of::<LeapInfo>() != 0 {
            return None;
        }

        let mut elements = Vec::<LeapInfo>::with_capacity(count);

        let mut new_byte_size = byte_size;
        let status = RegQueryValueExW(
            leap_sec_key.0,
            reg_subkey_name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            elements.as_mut_ptr().cast(),
            &mut new_byte_size,
        );
        if status != ERROR_SUCCESS as LSTATUS || new_byte_size != byte_size {
            return None;
        }
        elements.set_len(count);

        // The Windows Registry started tracking leap seconds since June 2018.
        // The initial difference between TAI and UTC is therefore 37 seconds.
        let mut diff = 37;
        let mut list = Vec::new();
        for element in elements {
            let Ok(year) = element.year.try_into() else { continue };
            let Ok(month) = u8::try_from(element.month) else { continue };
            let Ok(month) = month.try_into() else { continue };
            let Ok(day) = element.day.try_into() else { continue };
            let Ok(date) = time::Date::from_calendar_date(year, month, day) else { continue };
            let Ok(hour) = element.hour.try_into() else { continue };
            let Ok(date_time) = date.with_hms(hour, 59, 59) else { continue };
            let time_stamp = date_time.assume_utc().unix_timestamp() + 1;
            if element.negative != 0 {
                diff -= 1;
            } else {
                diff += 1;
            }
            if time_stamp >= EXPIRES_AT_UTC {
                list.push((time_stamp, diff));
            }
        }

        Some(list.into())
    }
}

#[cfg(all(feature = "std", unix))]
fn read_additional_leap_seconds() -> Option<Box<[(i64, i64)]>> {
    use time::Month;

    let file = std::fs::read_to_string("/usr/share/zoneinfo/leapseconds").ok()?;
    let mut elements = Vec::new();
    let mut diff = FIRST_LEAP_SECONDS_DIFF;
    for line in file.split('\n') {
        let Some(rem) = line.strip_prefix("Leap\t") else { continue };

        let Some((year, rem)) = rem.split_once('\t') else { continue };
        let Ok(year) = year.parse() else { continue };
        let Some((month, rem)) = rem.split_once('\t') else { continue };
        let month = match month {
            "Jan" => Month::January,
            "Feb" => Month::February,
            "Mar" => Month::March,
            "Apr" => Month::April,
            "May" => Month::May,
            "Jun" => Month::June,
            "Jul" => Month::July,
            "Aug" => Month::August,
            "Sep" => Month::September,
            "Oct" => Month::October,
            "Nov" => Month::November,
            "Dec" => Month::December,
            _ => continue,
        };
        let Some((day, rem)) = rem.split_once('\t') else { continue };
        let Ok(day) = day.parse() else { continue };
        let Ok(date) = time::Date::from_calendar_date(year, month, day) else { continue };

        let Some((hour, rem)) = rem.split_once(':') else { continue };
        let Ok(hour) = hour.parse() else { continue };
        let Some((minute, rem)) = rem.split_once(':') else { continue };
        let Ok(minute) = minute.parse() else { continue };
        let Some((second, rem)) = rem.split_once('\t') else { continue };
        let Ok(second) = second.parse() else { continue };
        let Ok(date_time) = date.with_hms(hour, minute, u8::min(second, 59)) else { continue };
        let mut time_stamp = date_time.assume_utc().unix_timestamp();

        let Some((plus_minus, _)) = rem.split_once('\t') else { continue };
        match plus_minus {
            "+" => {
                time_stamp += 1;
                diff += 1
            }
            "-" => diff -= 1,
            _ => continue,
        }

        if time_stamp >= EXPIRES_AT_UTC {
            elements.push((time_stamp, diff));
        }
    }
    Some(elements.into())
}

#[cfg(all(feature = "std", any(windows, unix)))]
static ADDITIONAL_LEAP_SECONDS: once_cell::sync::OnceCell<Box<[(i64, i64)]>> =
    once_cell::sync::OnceCell::new();

impl From<OffsetDateTime> for TaiDateTime {
    fn from(time: OffsetDateTime) -> Self {
        let unix_time_stamp = time - OffsetDateTime::UNIX_EPOCH;

        #[cfg(all(feature = "std", any(windows, unix)))]
        if unix_time_stamp.whole_seconds() >= EXPIRES_AT_UTC {
            let leap_seconds = ADDITIONAL_LEAP_SECONDS
                .get_or_init(|| read_additional_leap_seconds().unwrap_or_default());

            if let Some((_, diff)) = leap_seconds
                .iter()
                .cloned()
                .rev()
                .find(|&(t, _)| t <= unix_time_stamp.whole_seconds())
            {
                return Self(unix_time_stamp + Duration::new(diff, 0));
            }
        }

        let (_, diff) = LEAP_SECONDS
            .iter()
            .cloned()
            .rev()
            .find(|&(t, _)| t <= unix_time_stamp.whole_seconds())
            .unwrap_or((0, FIRST_LEAP_SECONDS_DIFF));

        Self(unix_time_stamp + Duration::new(diff, 0))
    }
}

impl From<TaiDateTime> for OffsetDateTime {
    fn from(time: TaiDateTime) -> Self {
        #[cfg(all(feature = "std", any(windows, unix)))]
        if time.0.whole_seconds() >= EXPIRES_AT_TAI {
            let leap_seconds = ADDITIONAL_LEAP_SECONDS
                .get_or_init(|| read_additional_leap_seconds().unwrap_or_default());

            if let Some((_, diff)) = leap_seconds
                .iter()
                .cloned()
                .rev()
                .find(|&(t, diff)| t + diff <= time.0.whole_seconds())
            {
                return OffsetDateTime::UNIX_EPOCH + (time.0 - Duration::new(diff, 0));
            }
        }

        let (_, diff) = LEAP_SECONDS
            .iter()
            .cloned()
            .rev()
            .find(|&(t, diff)| t + diff <= time.0.whole_seconds())
            .unwrap_or((0, FIRST_LEAP_SECONDS_DIFF));

        OffsetDateTime::UNIX_EPOCH + (time.0 - Duration::new(diff, 0))
    }
}
