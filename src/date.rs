use std::fmt;
use failure::Error;
use termion::{color, cursor};


#[derive(Eq, PartialEq, Debug, Ord, PartialOrd, Clone)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}/{:02}/{:02}", self.year, self.month, self.day)
    }
}

impl Date {
    pub fn parse_mdy(s: &str) -> Result<Date, Error> {
        let month = s[0..2].parse::<u8>()?;
        let day = s[3..5].parse::<u8>()?;
        let year = s[6..8].parse::<i32>()?;
        let year = if year > 50 { year + 1900 } else { year + 2000 };
        Ok(Date {
            year: year,
            month: month,
            day: day,
        })
    }

    pub fn parse_ymd(s: &str) -> Result<Date, Error> {
        let year = s[0..4].parse::<i32>()?;
        let month = s[5..7].parse::<u8>()?;
        let day = s[8..10].parse::<u8>()?;

        Ok(Date {
            year: year,
            month: month,
            day: day,
        })
    }
}

#[cfg(test)]
mod tests {
    use date::Date;
    #[test]
    fn parse_mdy() {
        assert_eq!(Date {
                       year: 2017,
                       month: 10,
                       day: 3,
                   },
                   Date::parse_mdy("10/03/17").unwrap());
        assert_eq!(Date {
                       year: 1997,
                       month: 3,
                       day: 17,
                   },
                   Date::parse_mdy("03/17/97").unwrap());
    }
    fn parse_ymd() {
        assert_eq!(Date {
                       year: 2017,
                       month: 10,
                       day: 3,
                   },
                   Date::parse_mdy("2017/10/03").unwrap());
        assert_eq!(Date {
                       year: 1997,
                       month: 3,
                       day: 17,
                   },
                   Date::parse_mdy("1997/03/17").unwrap());
    }
}
