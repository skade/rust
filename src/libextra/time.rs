// Copyright 2012-2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[allow(missing_doc)];

use std::io;
use std::num;
use std::str;

static NSEC_PER_SEC: i32 = 1_000_000_000_i32;

pub mod rustrt {
    use super::Tm;

    #[abi = "cdecl"]
    extern {
        pub fn get_time(sec: &mut i64, nsec: &mut i32);
        pub fn precise_time_ns(ns: &mut u64);
        pub fn rust_tzset();
        pub fn rust_gmtime(sec: i64, nsec: i32, result: &mut Tm);
        pub fn rust_localtime(sec: i64, nsec: i32, result: &mut Tm);
        pub fn rust_timegm(tm: &Tm) -> i64;
        pub fn rust_mktime(tm: &Tm) -> i64;
    }
}

/// A record specifying a time value in seconds and nanoseconds.
#[deriving(Clone, DeepClone, Eq, Encodable, Decodable)]
pub struct Timespec { sec: i64, nsec: i32 }

/*
 * Timespec assumes that pre-epoch Timespecs have negative sec and positive
 * nsec fields. Darwin's and Linux's struct timespec functions handle pre-
 * epoch timestamps using a "two steps back, one step forward" representation,
 * though the man pages do not actually document this. For example, the time
 * -1.2 seconds before the epoch is represented by `Timespec { sec: -2_i64,
 * nsec: 800_000_000_i32 }`.
 */
impl Timespec {
    pub fn new(sec: i64, nsec: i32) -> Timespec {
        assert!(nsec >= 0 && nsec < NSEC_PER_SEC);
        Timespec { sec: sec, nsec: nsec }
    }
}

impl Ord for Timespec {
    fn lt(&self, other: &Timespec) -> bool {
        self.sec < other.sec ||
            (self.sec == other.sec && self.nsec < other.nsec)
    }
}

/**
 * Returns the current time as a `timespec` containing the seconds and
 * nanoseconds since 1970-01-01T00:00:00Z.
 */
pub fn get_time() -> Timespec {
    #[fixed_stack_segment]; #[inline(never)];

    unsafe {
        let mut sec = 0i64;
        let mut nsec = 0i32;
        rustrt::get_time(&mut sec, &mut nsec);
        return Timespec::new(sec, nsec);
    }
}


/**
 * Returns the current value of a high-resolution performance counter
 * in nanoseconds since an unspecified epoch.
 */
pub fn precise_time_ns() -> u64 {
    #[fixed_stack_segment]; #[inline(never)];

    unsafe {
        let mut ns = 0u64;
        rustrt::precise_time_ns(&mut ns);
        ns
    }
}


/**
 * Returns the current value of a high-resolution performance counter
 * in seconds since an unspecified epoch.
 */
pub fn precise_time_s() -> f64 {
    return (precise_time_ns() as f64) / 1000000000.;
}

pub fn tzset() {
    #[fixed_stack_segment]; #[inline(never)];

    unsafe {
        rustrt::rust_tzset();
    }
}

#[deriving(Clone, DeepClone, Eq, Encodable, Decodable)]
pub struct Tm {
    tm_sec: i32, // seconds after the minute ~[0-60]
    tm_min: i32, // minutes after the hour ~[0-59]
    tm_hour: i32, // hours after midnight ~[0-23]
    tm_mday: i32, // days of the month ~[1-31]
    tm_mon: i32, // months since January ~[0-11]
    tm_year: i32, // years since 1900
    tm_wday: i32, // days since Sunday ~[0-6]
    tm_yday: i32, // days since January 1 ~[0-365]
    tm_isdst: i32, // Daylight Savings Time flag
    tm_gmtoff: i32, // offset from UTC in seconds
    tm_zone: ~str, // timezone abbreviation
    tm_nsec: i32, // nanoseconds
}

pub fn empty_tm() -> Tm {
    // 64 is the max size of the timezone buffer allocated on windows
    // in rust_localtime. In glibc the max timezone size is supposedly 3.
    let zone = str::with_capacity(64);
    Tm {
        tm_sec: 0_i32,
        tm_min: 0_i32,
        tm_hour: 0_i32,
        tm_mday: 0_i32,
        tm_mon: 0_i32,
        tm_year: 0_i32,
        tm_wday: 0_i32,
        tm_yday: 0_i32,
        tm_isdst: 0_i32,
        tm_gmtoff: 0_i32,
        tm_zone: zone,
        tm_nsec: 0_i32,
    }
}

/// Returns the specified time in UTC
pub fn at_utc(clock: Timespec) -> Tm {
    #[fixed_stack_segment]; #[inline(never)];

    unsafe {
        let Timespec { sec, nsec } = clock;
        let mut tm = empty_tm();
        rustrt::rust_gmtime(sec, nsec, &mut tm);
        tm
    }
}

/// Returns the current time in UTC
pub fn now_utc() -> Tm {
    at_utc(get_time())
}

/// Returns the specified time in the local timezone
pub fn at(clock: Timespec) -> Tm {
    #[fixed_stack_segment]; #[inline(never)];

    unsafe {
        let Timespec { sec, nsec } = clock;
        let mut tm = empty_tm();
        rustrt::rust_localtime(sec, nsec, &mut tm);
        tm
    }
}

/// Returns the current time in the local timezone
pub fn now() -> Tm {
    at(get_time())
}

/// Parses the time from the string according to the format string.
pub fn strptime(s: &str, format: &str) -> Result<Tm, ~str> {
    do_strptime(s, format)
}

/// Formats the time according to the format string.
pub fn strftime(format: &str, tm: &Tm) -> ~str {
    do_strftime(format, tm)
}

impl Tm {
    /// Convert time to the seconds from January 1, 1970
    pub fn to_timespec(&self) -> Timespec {
        #[fixed_stack_segment]; #[inline(never)];

        unsafe {
            let sec = match self.tm_gmtoff {
                0_i32 => rustrt::rust_timegm(self),
                _     => rustrt::rust_mktime(self)
            };

            Timespec::new(sec, self.tm_nsec)
        }
    }

    /// Convert time to the local timezone
    pub fn to_local(&self) -> Tm {
        at(self.to_timespec())
    }

    /// Convert time to the UTC
    pub fn to_utc(&self) -> Tm {
        at_utc(self.to_timespec())
    }

    /**
     * Return a string of the current time in the form
     * "Thu Jan  1 00:00:00 1970".
     */
    pub fn ctime(&self) -> ~str { self.strftime("%c") }

    /// Formats the time according to the format string.
    pub fn strftime(&self, format: &str) -> ~str {
        strftime(format, self)
    }

    /**
     * Returns a time string formatted according to RFC 822.
     *
     * local: "Thu, 22 Mar 2012 07:53:18 PST"
     * utc:   "Thu, 22 Mar 2012 14:53:18 UTC"
     */
    pub fn rfc822(&self) -> ~str {
        if self.tm_gmtoff == 0_i32 {
            self.strftime("%a, %d %b %Y %T GMT")
        } else {
            self.strftime("%a, %d %b %Y %T %Z")
        }
    }

    /**
     * Returns a time string formatted according to RFC 822 with Zulu time.
     *
     * local: "Thu, 22 Mar 2012 07:53:18 -0700"
     * utc:   "Thu, 22 Mar 2012 14:53:18 -0000"
     */
    pub fn rfc822z(&self) -> ~str {
        self.strftime("%a, %d %b %Y %T %z")
    }

    /**
     * Returns a time string formatted according to ISO 8601.
     *
     * local: "2012-02-22T07:53:18-07:00"
     * utc:   "2012-02-22T14:53:18Z"
     */
    pub fn rfc3339(&self) -> ~str {
        if self.tm_gmtoff == 0_i32 {
            self.strftime("%Y-%m-%dT%H:%M:%SZ")
        } else {
            let s = self.strftime("%Y-%m-%dT%H:%M:%S");
            let sign = if self.tm_gmtoff > 0_i32 { '+' } else { '-' };
            let mut m = num::abs(self.tm_gmtoff) / 60_i32;
            let h = m / 60_i32;
            m -= h * 60_i32;
            s + format!("{}{:02d}:{:02d}", sign, h as int, m as int)
        }
    }
}

fn do_strptime(s: &str, format: &str) -> Result<Tm, ~str> {
    fn match_str(s: &str, pos: uint, needle: &str) -> bool {
        let mut i = pos;
        for ch in needle.byte_iter() {
            if s[i] != ch {
                return false;
            }
            i += 1u;
        }
        return true;
    }

    fn match_strs(ss: &str, pos: uint, strs: &[(~str, i32)])
      -> Option<(i32, uint)> {
        let mut i = 0u;
        let len = strs.len();
        while i < len {
            match strs[i] { // can't use let due to let-pattern bugs
                (ref needle, value) => {
                    if match_str(ss, pos, *needle) {
                        return Some((value, pos + needle.len()));
                    }
                }
            }
            i += 1u;
        }

        None
    }

    fn match_digits(ss: &str, pos: uint, digits: uint, ws: bool)
      -> Option<(i32, uint)> {
        let mut pos = pos;
        let len = ss.len();
        let mut value = 0_i32;

        let mut i = 0u;
        while i < digits {
            if pos >= len {
                return None;
            }
            let range = ss.char_range_at(pos);
            pos = range.next;

            match range.ch {
              '0' .. '9' => {
                value = value * 10_i32 + (range.ch as i32 - '0' as i32);
              }
              ' ' if ws => (),
              _ => return None
            }
            i += 1u;
        }

        Some((value, pos))
    }

    fn match_fractional_seconds(ss: &str, pos: uint) -> (i32, uint) {
        let len = ss.len();
        let mut value = 0_i32;
        let mut multiplier = NSEC_PER_SEC / 10;
        let mut pos = pos;

        loop {
            if pos >= len {
                break;
            }
            let range = ss.char_range_at(pos);

            match range.ch {
                '0' .. '9' => {
                    pos = range.next;
                    // This will drop digits after the nanoseconds place
                    let digit = range.ch as i32 - '0' as i32;
                    value += digit * multiplier;
                    multiplier /= 10;
                }
                _ => break
            }
        }

        (value, pos)
    }

    fn match_digits_in_range(ss: &str, pos: uint, digits: uint, ws: bool,
                             min: i32, max: i32) -> Option<(i32, uint)> {
        match match_digits(ss, pos, digits, ws) {
          Some((val, pos)) if val >= min && val <= max => {
            Some((val, pos))
          }
          _ => None
        }
    }

    fn parse_char(s: &str, pos: uint, c: char) -> Result<uint, ~str> {
        let range = s.char_range_at(pos);

        if c == range.ch {
            Ok(range.next)
        } else {
            Err(format!("Expected {}, found {}",
                str::from_char(c),
                str::from_char(range.ch)))
        }
    }

    fn parse_type(s: &str, pos: uint, ch: char, tm: &mut Tm)
      -> Result<uint, ~str> {
        match ch {
          'A' => match match_strs(s, pos, [
              (~"Sunday", 0_i32),
              (~"Monday", 1_i32),
              (~"Tuesday", 2_i32),
              (~"Wednesday", 3_i32),
              (~"Thursday", 4_i32),
              (~"Friday", 5_i32),
              (~"Saturday", 6_i32)
          ]) {
            Some(item) => { let (v, pos) = item; tm.tm_wday = v; Ok(pos) }
            None => Err(~"Invalid day")
          },
          'a' => match match_strs(s, pos, [
              (~"Sun", 0_i32),
              (~"Mon", 1_i32),
              (~"Tue", 2_i32),
              (~"Wed", 3_i32),
              (~"Thu", 4_i32),
              (~"Fri", 5_i32),
              (~"Sat", 6_i32)
          ]) {
            Some(item) => { let (v, pos) = item; tm.tm_wday = v; Ok(pos) }
            None => Err(~"Invalid day")
          },
          'B' => match match_strs(s, pos, [
              (~"January", 0_i32),
              (~"February", 1_i32),
              (~"March", 2_i32),
              (~"April", 3_i32),
              (~"May", 4_i32),
              (~"June", 5_i32),
              (~"July", 6_i32),
              (~"August", 7_i32),
              (~"September", 8_i32),
              (~"October", 9_i32),
              (~"November", 10_i32),
              (~"December", 11_i32)
          ]) {
            Some(item) => { let (v, pos) = item; tm.tm_mon = v; Ok(pos) }
            None => Err(~"Invalid month")
          },
          'b' | 'h' => match match_strs(s, pos, [
              (~"Jan", 0_i32),
              (~"Feb", 1_i32),
              (~"Mar", 2_i32),
              (~"Apr", 3_i32),
              (~"May", 4_i32),
              (~"Jun", 5_i32),
              (~"Jul", 6_i32),
              (~"Aug", 7_i32),
              (~"Sep", 8_i32),
              (~"Oct", 9_i32),
              (~"Nov", 10_i32),
              (~"Dec", 11_i32)
          ]) {
            Some(item) => { let (v, pos) = item; tm.tm_mon = v; Ok(pos) }
            None => Err(~"Invalid month")
          },
          'C' => match match_digits_in_range(s, pos, 2u, false, 0_i32,
                                             99_i32) {
            Some(item) => {
                let (v, pos) = item;
                  tm.tm_year += (v * 100_i32) - 1900_i32;
                  Ok(pos)
              }
            None => Err(~"Invalid year")
          },
          'c' => {
            parse_type(s, pos, 'a', &mut *tm)
                .and_then(|pos| parse_char(s, pos, ' '))
                .and_then(|pos| parse_type(s, pos, 'b', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ' '))
                .and_then(|pos| parse_type(s, pos, 'e', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ' '))
                .and_then(|pos| parse_type(s, pos, 'T', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ' '))
                .and_then(|pos| parse_type(s, pos, 'Y', &mut *tm))
          }
          'D' | 'x' => {
            parse_type(s, pos, 'm', &mut *tm)
                .and_then(|pos| parse_char(s, pos, '/'))
                .and_then(|pos| parse_type(s, pos, 'd', &mut *tm))
                .and_then(|pos| parse_char(s, pos, '/'))
                .and_then(|pos| parse_type(s, pos, 'y', &mut *tm))
          }
          'd' => match match_digits_in_range(s, pos, 2u, false, 1_i32,
                                             31_i32) {
            Some(item) => { let (v, pos) = item; tm.tm_mday = v; Ok(pos) }
            None => Err(~"Invalid day of the month")
          },
          'e' => match match_digits_in_range(s, pos, 2u, true, 1_i32,
                                             31_i32) {
            Some(item) => { let (v, pos) = item; tm.tm_mday = v; Ok(pos) }
            None => Err(~"Invalid day of the month")
          },
          'f' => {
            let (val, pos) = match_fractional_seconds(s, pos);
            tm.tm_nsec = val;
            Ok(pos)
          }
          'F' => {
            parse_type(s, pos, 'Y', &mut *tm)
                .and_then(|pos| parse_char(s, pos, '-'))
                .and_then(|pos| parse_type(s, pos, 'm', &mut *tm))
                .and_then(|pos| parse_char(s, pos, '-'))
                .and_then(|pos| parse_type(s, pos, 'd', &mut *tm))
          }
          'H' => {
            match match_digits_in_range(s, pos, 2u, false, 0_i32, 23_i32) {
              Some(item) => { let (v, pos) = item; tm.tm_hour = v; Ok(pos) }
              None => Err(~"Invalid hour")
            }
          }
          'I' => {
            match match_digits_in_range(s, pos, 2u, false, 1_i32, 12_i32) {
              Some(item) => {
                  let (v, pos) = item;
                  tm.tm_hour = if v == 12_i32 { 0_i32 } else { v };
                  Ok(pos)
              }
              None => Err(~"Invalid hour")
            }
          }
          'j' => {
            match match_digits_in_range(s, pos, 3u, false, 1_i32, 366_i32) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_yday = v - 1_i32;
                Ok(pos)
              }
              None => Err(~"Invalid day of year")
            }
          }
          'k' => {
            match match_digits_in_range(s, pos, 2u, true, 0_i32, 23_i32) {
              Some(item) => { let (v, pos) = item; tm.tm_hour = v; Ok(pos) }
              None => Err(~"Invalid hour")
            }
          }
          'l' => {
            match match_digits_in_range(s, pos, 2u, true, 1_i32, 12_i32) {
              Some(item) => {
                  let (v, pos) = item;
                  tm.tm_hour = if v == 12_i32 { 0_i32 } else { v };
                  Ok(pos)
              }
              None => Err(~"Invalid hour")
            }
          }
          'M' => {
            match match_digits_in_range(s, pos, 2u, false, 0_i32, 59_i32) {
              Some(item) => { let (v, pos) = item; tm.tm_min = v; Ok(pos) }
              None => Err(~"Invalid minute")
            }
          }
          'm' => {
            match match_digits_in_range(s, pos, 2u, false, 1_i32, 12_i32) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_mon = v - 1_i32;
                Ok(pos)
              }
              None => Err(~"Invalid month")
            }
          }
          'n' => parse_char(s, pos, '\n'),
          'P' => match match_strs(s, pos,
                                  [(~"am", 0_i32), (~"pm", 12_i32)]) {

            Some(item) => { let (v, pos) = item; tm.tm_hour += v; Ok(pos) }
            None => Err(~"Invalid hour")
          },
          'p' => match match_strs(s, pos,
                                  [(~"AM", 0_i32), (~"PM", 12_i32)]) {

            Some(item) => { let (v, pos) = item; tm.tm_hour += v; Ok(pos) }
            None => Err(~"Invalid hour")
          },
          'R' => {
            parse_type(s, pos, 'H', &mut *tm)
                .and_then(|pos| parse_char(s, pos, ':'))
                .and_then(|pos| parse_type(s, pos, 'M', &mut *tm))
          }
          'r' => {
            parse_type(s, pos, 'I', &mut *tm)
                .and_then(|pos| parse_char(s, pos, ':'))
                .and_then(|pos| parse_type(s, pos, 'M', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ':'))
                .and_then(|pos| parse_type(s, pos, 'S', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ' '))
                .and_then(|pos| parse_type(s, pos, 'p', &mut *tm))
          }
          'S' => {
            match match_digits_in_range(s, pos, 2u, false, 0_i32, 60_i32) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_sec = v;
                Ok(pos)
              }
              None => Err(~"Invalid second")
            }
          }
          //'s' {}
          'T' | 'X' => {
            parse_type(s, pos, 'H', &mut *tm)
                .and_then(|pos| parse_char(s, pos, ':'))
                .and_then(|pos| parse_type(s, pos, 'M', &mut *tm))
                .and_then(|pos| parse_char(s, pos, ':'))
                .and_then(|pos| parse_type(s, pos, 'S', &mut *tm))
          }
          't' => parse_char(s, pos, '\t'),
          'u' => {
            match match_digits_in_range(s, pos, 1u, false, 1_i32, 7_i32) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_wday = if v == 7 { 0 } else { v };
                Ok(pos)
              }
              None => Err(~"Invalid day of week")
            }
          }
          'v' => {
            parse_type(s, pos, 'e', &mut *tm)
                .and_then(|pos|  parse_char(s, pos, '-'))
                .and_then(|pos| parse_type(s, pos, 'b', &mut *tm))
                .and_then(|pos| parse_char(s, pos, '-'))
                .and_then(|pos| parse_type(s, pos, 'Y', &mut *tm))
          }
          //'W' {}
          'w' => {
            match match_digits_in_range(s, pos, 1u, false, 0_i32, 6_i32) {
              Some(item) => { let (v, pos) = item; tm.tm_wday = v; Ok(pos) }
              None => Err(~"Invalid day of week")
            }
          }
          //'X' {}
          //'x' {}
          'Y' => {
            match match_digits(s, pos, 4u, false) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_year = v - 1900_i32;
                Ok(pos)
              }
              None => Err(~"Invalid year")
            }
          }
          'y' => {
            match match_digits_in_range(s, pos, 2u, false, 0_i32, 99_i32) {
              Some(item) => {
                let (v, pos) = item;
                tm.tm_year = v;
                Ok(pos)
              }
              None => Err(~"Invalid year")
            }
          }
          'Z' => {
            if match_str(s, pos, "UTC") || match_str(s, pos, "GMT") {
                tm.tm_gmtoff = 0_i32;
                tm.tm_zone = ~"UTC";
                Ok(pos + 3u)
            } else {
                // It's odd, but to maintain compatibility with c's
                // strptime we ignore the timezone.
                let mut pos = pos;
                let len = s.len();
                while pos < len {
                    let range = s.char_range_at(pos);
                    pos = range.next;
                    if range.ch == ' ' { break; }
                }

                Ok(pos)
            }
          }
          'z' => {
            let range = s.char_range_at(pos);

            if range.ch == '+' || range.ch == '-' {
                match match_digits(s, range.next, 4u, false) {
                  Some(item) => {
                    let (v, pos) = item;
                    if v == 0_i32 {
                        tm.tm_gmtoff = 0_i32;
                        tm.tm_zone = ~"UTC";
                    }

                    Ok(pos)
                  }
                  None => Err(~"Invalid zone offset")
                }
            } else {
                Err(~"Invalid zone offset")
            }
          }
          '%' => parse_char(s, pos, '%'),
          ch => {
            Err(format!("unknown formatting type: {}", str::from_char(ch)))
          }
        }
    }

    do io::with_str_reader(format) |rdr| {
        let mut tm = Tm {
            tm_sec: 0_i32,
            tm_min: 0_i32,
            tm_hour: 0_i32,
            tm_mday: 0_i32,
            tm_mon: 0_i32,
            tm_year: 0_i32,
            tm_wday: 0_i32,
            tm_yday: 0_i32,
            tm_isdst: 0_i32,
            tm_gmtoff: 0_i32,
            tm_zone: ~"",
            tm_nsec: 0_i32,
        };
        let mut pos = 0u;
        let len = s.len();
        let mut result = Err(~"Invalid time");

        while !rdr.eof() && pos < len {
            let range = s.char_range_at(pos);
            let ch = range.ch;
            let next = range.next;

            match rdr.read_char() {
                '%' => {
                    match parse_type(s, pos, rdr.read_char(), &mut tm) {
                        Ok(next) => pos = next,
                        Err(e) => { result = Err(e); break; }
                    }
                },
                c => {
                    if c != ch { break }
                    pos = next;
                }
            }
        }

        if pos == len && rdr.eof() {
            Ok(Tm {
                tm_sec: tm.tm_sec,
                tm_min: tm.tm_min,
                tm_hour: tm.tm_hour,
                tm_mday: tm.tm_mday,
                tm_mon: tm.tm_mon,
                tm_year: tm.tm_year,
                tm_wday: tm.tm_wday,
                tm_yday: tm.tm_yday,
                tm_isdst: tm.tm_isdst,
                tm_gmtoff: tm.tm_gmtoff,
                tm_zone: tm.tm_zone.clone(),
                tm_nsec: tm.tm_nsec,
            })
        } else { result }
    }
}

fn do_strftime(format: &str, tm: &Tm) -> ~str {
    fn parse_type(ch: char, tm: &Tm) -> ~str {
        //FIXME (#2350): Implement missing types.
      let die = || format!("strftime: can't understand this format {} ", ch);
        match ch {
          'A' => match tm.tm_wday as int {
            0 => ~"Sunday",
            1 => ~"Monday",
            2 => ~"Tuesday",
            3 => ~"Wednesday",
            4 => ~"Thursday",
            5 => ~"Friday",
            6 => ~"Saturday",
            _ => die()
          },
         'a' => match tm.tm_wday as int {
            0 => ~"Sun",
            1 => ~"Mon",
            2 => ~"Tue",
            3 => ~"Wed",
            4 => ~"Thu",
            5 => ~"Fri",
            6 => ~"Sat",
            _ => die()
          },
          'B' => match tm.tm_mon as int {
            0 => ~"January",
            1 => ~"February",
            2 => ~"March",
            3 => ~"April",
            4 => ~"May",
            5 => ~"June",
            6 => ~"July",
            7 => ~"August",
            8 => ~"September",
            9 => ~"October",
            10 => ~"November",
            11 => ~"December",
            _ => die()
          },
          'b' | 'h' => match tm.tm_mon as int {
            0 => ~"Jan",
            1 => ~"Feb",
            2 => ~"Mar",
            3 => ~"Apr",
            4 => ~"May",
            5 => ~"Jun",
            6 => ~"Jul",
            7 => ~"Aug",
            8 => ~"Sep",
            9 => ~"Oct",
            10 => ~"Nov",
            11 => ~"Dec",
            _  => die()
          },
          'C' => format!("{:02d}", (tm.tm_year as int + 1900) / 100),
          'c' => {
            format!("{} {} {} {} {}",
                parse_type('a', tm),
                parse_type('b', tm),
                parse_type('e', tm),
                parse_type('T', tm),
                parse_type('Y', tm))
          }
          'D' | 'x' => {
            format!("{}/{}/{}",
                parse_type('m', tm),
                parse_type('d', tm),
                parse_type('y', tm))
          }
          'd' => format!("{:02d}", tm.tm_mday),
          'e' => format!("{:2d}", tm.tm_mday),
          'f' => format!("{:09d}", tm.tm_nsec),
          'F' => {
            format!("{}-{}-{}",
                parse_type('Y', tm),
                parse_type('m', tm),
                parse_type('d', tm))
          }
          //'G' {}
          //'g' {}
          'H' => format!("{:02d}", tm.tm_hour),
          'I' => {
            let mut h = tm.tm_hour;
            if h == 0 { h = 12 }
            if h > 12 { h -= 12 }
            format!("{:02d}", h)
          }
          'j' => format!("{:03d}", tm.tm_yday + 1),
          'k' => format!("{:2d}", tm.tm_hour),
          'l' => {
            let mut h = tm.tm_hour;
            if h == 0 { h = 12 }
            if h > 12 { h -= 12 }
            format!("{:2d}", h)
          }
          'M' => format!("{:02d}", tm.tm_min),
          'm' => format!("{:02d}", tm.tm_mon + 1),
          'n' => ~"\n",
          'P' => if (tm.tm_hour as int) < 12 { ~"am" } else { ~"pm" },
          'p' => if (tm.tm_hour as int) < 12 { ~"AM" } else { ~"PM" },
          'R' => {
            format!("{}:{}",
                parse_type('H', tm),
                parse_type('M', tm))
          }
          'r' => {
            format!("{}:{}:{} {}",
                parse_type('I', tm),
                parse_type('M', tm),
                parse_type('S', tm),
                parse_type('p', tm))
          }
          'S' => format!("{:02d}", tm.tm_sec),
          's' => format!("{}", tm.to_timespec().sec),
          'T' | 'X' => {
            format!("{}:{}:{}",
                parse_type('H', tm),
                parse_type('M', tm),
                parse_type('S', tm))
          }
          't' => ~"\t",
          //'U' {}
          'u' => {
            let i = tm.tm_wday as int;
            (if i == 0 { 7 } else { i }).to_str()
          }
          //'V' {}
          'v' => {
            format!("{}-{}-{}",
                parse_type('e', tm),
                parse_type('b', tm),
                parse_type('Y', tm))
          }
          //'W' {}
          'w' => (tm.tm_wday as int).to_str(),
          //'X' {}
          //'x' {}
          'Y' => (tm.tm_year as int + 1900).to_str(),
          'y' => format!("{:02d}", (tm.tm_year as int + 1900) % 100),
          'Z' => tm.tm_zone.clone(),
          'z' => {
            let sign = if tm.tm_gmtoff > 0_i32 { '+' } else { '-' };
            let mut m = num::abs(tm.tm_gmtoff) / 60_i32;
            let h = m / 60_i32;
            m -= h * 60_i32;
            format!("{}{:02d}{:02d}", sign, h, m)
          }
          //'+' {}
          '%' => ~"%",
          _   => die()
        }
    }

    let mut buf = ~"";

    do io::with_str_reader(format) |rdr| {
        while !rdr.eof() {
            match rdr.read_char() {
                '%' => buf.push_str(parse_type(rdr.read_char(), tm)),
                ch => buf.push_char(ch)
            }
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64;
    use std::os;
    use std::result::{Err, Ok};

    fn test_get_time() {
        static SOME_RECENT_DATE: i64 = 1325376000i64; // 2012-01-01T00:00:00Z
        static SOME_FUTURE_DATE: i64 = 1577836800i64; // 2020-01-01T00:00:00Z

        let tv1 = get_time();
        debug2!("tv1={:?} sec + {:?} nsec", tv1.sec as uint, tv1.nsec as uint);

        assert!(tv1.sec > SOME_RECENT_DATE);
        assert!(tv1.nsec < 1000000000i32);

        let tv2 = get_time();
        debug2!("tv2={:?} sec + {:?} nsec", tv2.sec as uint, tv2.nsec as uint);

        assert!(tv2.sec >= tv1.sec);
        assert!(tv2.sec < SOME_FUTURE_DATE);
        assert!(tv2.nsec < 1000000000i32);
        if tv2.sec == tv1.sec {
            assert!(tv2.nsec >= tv1.nsec);
        }
    }

    fn test_precise_time() {
        let s0 = precise_time_s();
        let ns1 = precise_time_ns();

        debug2!("s0={} sec", f64::to_str_digits(s0, 9u));
        assert!(s0 > 0.);
        let ns0 = (s0 * 1000000000.) as u64;
        debug2!("ns0={:?} ns", ns0);

        debug2!("ns1={:?} ns", ns0);
        assert!(ns1 >= ns0);

        let ns2 = precise_time_ns();
        debug2!("ns2={:?} ns", ns0);
        assert!(ns2 >= ns1);
    }

    fn test_at_utc() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let utc = at_utc(time);

        assert!(utc.tm_sec == 30_i32);
        assert!(utc.tm_min == 31_i32);
        assert!(utc.tm_hour == 23_i32);
        assert!(utc.tm_mday == 13_i32);
        assert!(utc.tm_mon == 1_i32);
        assert!(utc.tm_year == 109_i32);
        assert!(utc.tm_wday == 5_i32);
        assert!(utc.tm_yday == 43_i32);
        assert!(utc.tm_isdst == 0_i32);
        assert!(utc.tm_gmtoff == 0_i32);
        assert!(utc.tm_zone == ~"UTC");
        assert!(utc.tm_nsec == 54321_i32);
    }

    fn test_at() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let local = at(time);

        error2!("time_at: {:?}", local);

        assert!(local.tm_sec == 30_i32);
        assert!(local.tm_min == 31_i32);
        assert!(local.tm_hour == 15_i32);
        assert!(local.tm_mday == 13_i32);
        assert!(local.tm_mon == 1_i32);
        assert!(local.tm_year == 109_i32);
        assert!(local.tm_wday == 5_i32);
        assert!(local.tm_yday == 43_i32);
        assert!(local.tm_isdst == 0_i32);
        assert!(local.tm_gmtoff == -28800_i32);

        // FIXME (#2350): We should probably standardize on the timezone
        // abbreviation.
        let zone = &local.tm_zone;
        assert!(*zone == ~"PST" || *zone == ~"Pacific Standard Time");

        assert!(local.tm_nsec == 54321_i32);
    }

    fn test_to_timespec() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let utc = at_utc(time);

        assert_eq!(utc.to_timespec(), time);
        assert_eq!(utc.to_local().to_timespec(), time);
    }

    fn test_conversions() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let utc = at_utc(time);
        let local = at(time);

        assert!(local.to_local() == local);
        assert!(local.to_utc() == utc);
        assert!(local.to_utc().to_local() == local);
        assert!(utc.to_utc() == utc);
        assert!(utc.to_local() == local);
        assert!(utc.to_local().to_utc() == utc);
    }

    fn test_strptime() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        match strptime("", "") {
          Ok(ref tm) => {
            assert!(tm.tm_sec == 0_i32);
            assert!(tm.tm_min == 0_i32);
            assert!(tm.tm_hour == 0_i32);
            assert!(tm.tm_mday == 0_i32);
            assert!(tm.tm_mon == 0_i32);
            assert!(tm.tm_year == 0_i32);
            assert!(tm.tm_wday == 0_i32);
            assert!(tm.tm_isdst == 0_i32);
            assert!(tm.tm_gmtoff == 0_i32);
            assert!(tm.tm_zone == ~"");
            assert!(tm.tm_nsec == 0_i32);
          }
          Err(_) => ()
        }

        let format = "%a %b %e %T.%f %Y";
        assert_eq!(strptime("", format), Err(~"Invalid time"));
        assert!(strptime("Fri Feb 13 15:31:30", format)
            == Err(~"Invalid time"));

        match strptime("Fri Feb 13 15:31:30.01234 2009", format) {
          Err(e) => fail2!(e),
          Ok(ref tm) => {
            assert!(tm.tm_sec == 30_i32);
            assert!(tm.tm_min == 31_i32);
            assert!(tm.tm_hour == 15_i32);
            assert!(tm.tm_mday == 13_i32);
            assert!(tm.tm_mon == 1_i32);
            assert!(tm.tm_year == 109_i32);
            assert!(tm.tm_wday == 5_i32);
            assert!(tm.tm_yday == 0_i32);
            assert!(tm.tm_isdst == 0_i32);
            assert!(tm.tm_gmtoff == 0_i32);
            assert!(tm.tm_zone == ~"");
            assert!(tm.tm_nsec == 12340000_i32);
          }
        }

        fn test(s: &str, format: &str) -> bool {
            match strptime(s, format) {
              Ok(ref tm) => tm.strftime(format) == s.to_owned(),
              Err(e) => fail2!(e)
            }
        }

        let days = [
            ~"Sunday",
            ~"Monday",
            ~"Tuesday",
            ~"Wednesday",
            ~"Thursday",
            ~"Friday",
            ~"Saturday"
        ];
        for day in days.iter() {
            assert!(test(*day, "%A"));
        }

        let days = [
            ~"Sun",
            ~"Mon",
            ~"Tue",
            ~"Wed",
            ~"Thu",
            ~"Fri",
            ~"Sat"
        ];
        for day in days.iter() {
            assert!(test(*day, "%a"));
        }

        let months = [
            ~"January",
            ~"February",
            ~"March",
            ~"April",
            ~"May",
            ~"June",
            ~"July",
            ~"August",
            ~"September",
            ~"October",
            ~"November",
            ~"December"
        ];
        for day in months.iter() {
            assert!(test(*day, "%B"));
        }

        let months = [
            ~"Jan",
            ~"Feb",
            ~"Mar",
            ~"Apr",
            ~"May",
            ~"Jun",
            ~"Jul",
            ~"Aug",
            ~"Sep",
            ~"Oct",
            ~"Nov",
            ~"Dec"
        ];
        for day in months.iter() {
            assert!(test(*day, "%b"));
        }

        assert!(test("19", "%C"));
        assert!(test("Fri Feb 13 23:31:30 2009", "%c"));
        assert!(test("02/13/09", "%D"));
        assert!(test("03", "%d"));
        assert!(test("13", "%d"));
        assert!(test(" 3", "%e"));
        assert!(test("13", "%e"));
        assert!(test("2009-02-13", "%F"));
        assert!(test("03", "%H"));
        assert!(test("13", "%H"));
        assert!(test("03", "%I")); // FIXME (#2350): flesh out
        assert!(test("11", "%I")); // FIXME (#2350): flesh out
        assert!(test("044", "%j"));
        assert!(test(" 3", "%k"));
        assert!(test("13", "%k"));
        assert!(test(" 1", "%l"));
        assert!(test("11", "%l"));
        assert!(test("03", "%M"));
        assert!(test("13", "%M"));
        assert!(test("\n", "%n"));
        assert!(test("am", "%P"));
        assert!(test("pm", "%P"));
        assert!(test("AM", "%p"));
        assert!(test("PM", "%p"));
        assert!(test("23:31", "%R"));
        assert!(test("11:31:30 AM", "%r"));
        assert!(test("11:31:30 PM", "%r"));
        assert!(test("03", "%S"));
        assert!(test("13", "%S"));
        assert!(test("15:31:30", "%T"));
        assert!(test("\t", "%t"));
        assert!(test("1", "%u"));
        assert!(test("7", "%u"));
        assert!(test("13-Feb-2009", "%v"));
        assert!(test("0", "%w"));
        assert!(test("6", "%w"));
        assert!(test("2009", "%Y"));
        assert!(test("09", "%y"));
        assert!(strptime("UTC", "%Z").unwrap().tm_zone ==
            ~"UTC");
        assert!(strptime("PST", "%Z").unwrap().tm_zone ==
            ~"");
        assert!(strptime("-0000", "%z").unwrap().tm_gmtoff ==
            0);
        assert!(strptime("-0800", "%z").unwrap().tm_gmtoff ==
            0);
        assert!(test("%", "%%"));

        // Test for #7256
        assert_eq!(strptime("360", "%Y-%m-%d"), Err(~"Invalid year"))
    }

    fn test_ctime() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let utc   = at_utc(time);
        let local = at(time);

        error2!("test_ctime: {:?} {:?}", utc.ctime(), local.ctime());

        assert_eq!(utc.ctime(), ~"Fri Feb 13 23:31:30 2009");
        assert_eq!(local.ctime(), ~"Fri Feb 13 15:31:30 2009");
    }

    fn test_strftime() {
        os::setenv("TZ", "America/Los_Angeles");
        tzset();

        let time = Timespec::new(1234567890, 54321);
        let utc = at_utc(time);
        let local = at(time);

        assert_eq!(local.strftime(""), ~"");
        assert_eq!(local.strftime("%A"), ~"Friday");
        assert_eq!(local.strftime("%a"), ~"Fri");
        assert_eq!(local.strftime("%B"), ~"February");
        assert_eq!(local.strftime("%b"), ~"Feb");
        assert_eq!(local.strftime("%C"), ~"20");
        assert_eq!(local.strftime("%c"), ~"Fri Feb 13 15:31:30 2009");
        assert_eq!(local.strftime("%D"), ~"02/13/09");
        assert_eq!(local.strftime("%d"), ~"13");
        assert_eq!(local.strftime("%e"), ~"13");
        assert_eq!(local.strftime("%f"), ~"000054321");
        assert_eq!(local.strftime("%F"), ~"2009-02-13");
        // assert!(local.strftime("%G") == "2009");
        // assert!(local.strftime("%g") == "09");
        assert_eq!(local.strftime("%H"), ~"15");
        assert_eq!(local.strftime("%I"), ~"03");
        assert_eq!(local.strftime("%j"), ~"044");
        assert_eq!(local.strftime("%k"), ~"15");
        assert_eq!(local.strftime("%l"), ~" 3");
        assert_eq!(local.strftime("%M"), ~"31");
        assert_eq!(local.strftime("%m"), ~"02");
        assert_eq!(local.strftime("%n"), ~"\n");
        assert_eq!(local.strftime("%P"), ~"pm");
        assert_eq!(local.strftime("%p"), ~"PM");
        assert_eq!(local.strftime("%R"), ~"15:31");
        assert_eq!(local.strftime("%r"), ~"03:31:30 PM");
        assert_eq!(local.strftime("%S"), ~"30");
        assert_eq!(local.strftime("%s"), ~"1234567890");
        assert_eq!(local.strftime("%T"), ~"15:31:30");
        assert_eq!(local.strftime("%t"), ~"\t");
        // assert!(local.strftime("%U") == "06");
        assert_eq!(local.strftime("%u"), ~"5");
        // assert!(local.strftime("%V") == "07");
        assert_eq!(local.strftime("%v"), ~"13-Feb-2009");
        // assert!(local.strftime("%W") == "06");
        assert_eq!(local.strftime("%w"), ~"5");
        // handle "%X"
        // handle "%x"
        assert_eq!(local.strftime("%Y"), ~"2009");
        assert_eq!(local.strftime("%y"), ~"09");

        // FIXME (#2350): We should probably standardize on the timezone
        // abbreviation.
        let zone = local.strftime("%Z");
        assert!(zone == ~"PST" || zone == ~"Pacific Standard Time");

        assert_eq!(local.strftime("%z"), ~"-0800");
        assert_eq!(local.strftime("%%"), ~"%");

        // FIXME (#2350): We should probably standardize on the timezone
        // abbreviation.
        let rfc822 = local.rfc822();
        let prefix = ~"Fri, 13 Feb 2009 15:31:30 ";
        assert!(rfc822 == prefix + "PST" ||
                     rfc822 == prefix + "Pacific Standard Time");

        assert_eq!(local.ctime(), ~"Fri Feb 13 15:31:30 2009");
        assert_eq!(local.rfc822z(), ~"Fri, 13 Feb 2009 15:31:30 -0800");
        assert_eq!(local.rfc3339(), ~"2009-02-13T15:31:30-08:00");

        assert_eq!(utc.ctime(), ~"Fri Feb 13 23:31:30 2009");
        assert_eq!(utc.rfc822(), ~"Fri, 13 Feb 2009 23:31:30 GMT");
        assert_eq!(utc.rfc822z(), ~"Fri, 13 Feb 2009 23:31:30 -0000");
        assert_eq!(utc.rfc3339(), ~"2009-02-13T23:31:30Z");
    }

    fn test_timespec_eq_ord() {
        let a = &Timespec::new(-2, 1);
        let b = &Timespec::new(-1, 2);
        let c = &Timespec::new(1, 2);
        let d = &Timespec::new(2, 1);
        let e = &Timespec::new(2, 1);

        assert!(d.eq(e));
        assert!(c.ne(e));

        assert!(a.lt(b));
        assert!(b.lt(c));
        assert!(c.lt(d));

        assert!(a.le(b));
        assert!(b.le(c));
        assert!(c.le(d));
        assert!(d.le(e));
        assert!(e.le(d));

        assert!(b.ge(a));
        assert!(c.ge(b));
        assert!(d.ge(c));
        assert!(e.ge(d));
        assert!(d.ge(e));

        assert!(b.gt(a));
        assert!(c.gt(b));
        assert!(d.gt(c));
    }

    #[test]
    fn run_tests() {
        // The tests race on tzset. So instead of having many independent
        // tests, we will just call the functions now.
        test_get_time();
        test_precise_time();
        test_at_utc();
        test_at();
        test_to_timespec();
        test_conversions();
        test_strptime();
        test_ctime();
        test_strftime();
        test_timespec_eq_ord();
    }
}
