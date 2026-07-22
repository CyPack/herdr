use std::time::SystemTime;

use time::{OffsetDateTime, UtcOffset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileTimeSection {
    Future,
    Today,
    Yesterday,
    Previous7Days,
    Older,
    UnknownDate,
}

impl FileTimeSection {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Future => "Future",
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::Previous7Days => "Previous 7 Days",
            Self::Older => "Older",
            Self::UnknownDate => "Unknown Date",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileTimePresentation {
    pub section: FileTimeSection,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LocalCalendarAnchor {
    now: SystemTime,
    fixed_offset: Option<UtcOffset>,
}

#[cfg(test)]
std::thread_local! {
    static TEST_CALENDAR_ANCHOR: std::cell::Cell<Option<LocalCalendarAnchor>> =
        const { std::cell::Cell::new(None) };
}

impl LocalCalendarAnchor {
    pub(crate) fn now() -> Self {
        #[cfg(test)]
        if let Some(anchor) = TEST_CALENDAR_ANCHOR.with(std::cell::Cell::get) {
            return anchor;
        }
        Self {
            now: SystemTime::now(),
            fixed_offset: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_system_time(now: SystemTime) -> Self {
        Self {
            now,
            fixed_offset: None,
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_system_time_at_offset(
        now: SystemTime,
        fixed_offset: UtcOffset,
    ) -> Self {
        Self {
            now,
            fixed_offset: Some(fixed_offset),
        }
    }
}

/// Scope a deterministic calendar clock to the current test thread. The
/// production build has no override state, and nested/panicking test scopes
/// restore the prior anchor through the drop guard.
#[cfg(test)]
pub(crate) fn with_test_calendar_anchor<T>(
    anchor: LocalCalendarAnchor,
    run: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<LocalCalendarAnchor>);

    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_CALENDAR_ANCHOR.with(|slot| slot.set(self.0));
        }
    }

    let previous = TEST_CALENDAR_ANCHOR.with(|slot| slot.replace(Some(anchor)));
    let _restore = Restore(previous);
    run()
}

pub(crate) fn present_file_time(
    modified: Option<SystemTime>,
    anchor: LocalCalendarAnchor,
) -> FileTimePresentation {
    if let Some(fixed_offset) = anchor.fixed_offset {
        return present_file_time_with_resolver(modified, anchor, |_| Some(fixed_offset));
    }
    present_file_time_with_resolver(modified, anchor, |utc| UtcOffset::local_offset_at(utc).ok())
}

fn present_file_time_with_resolver(
    modified: Option<SystemTime>,
    anchor: LocalCalendarAnchor,
    resolve_offset: impl Fn(OffsetDateTime) -> Option<UtcOffset>,
) -> FileTimePresentation {
    let Some(modified) = modified else {
        return unknown_date();
    };
    let Some(anchor_local) = resolve_local(anchor.now, &resolve_offset) else {
        return unknown_date();
    };
    let Some(modified_local) = resolve_local(modified, &resolve_offset) else {
        return unknown_date();
    };

    let day_delta =
        i64::from(anchor_local.to_julian_day()) - i64::from(modified_local.to_julian_day());
    let section = match day_delta {
        ..=-1 => FileTimeSection::Future,
        0 => FileTimeSection::Today,
        1 => FileTimeSection::Yesterday,
        2..=7 => FileTimeSection::Previous7Days,
        _ => FileTimeSection::Older,
    };
    let label = match section {
        FileTimeSection::Future | FileTimeSection::Today | FileTimeSection::Yesterday => {
            clock_label(modified_local)
        }
        FileTimeSection::Previous7Days => format!(
            "{} {}",
            weekday_abbreviation(modified_local),
            clock_label(modified_local)
        ),
        FileTimeSection::Older if modified_local.year() == anchor_local.year() => format!(
            "{:02} {}",
            modified_local.day(),
            month_abbreviation(modified_local)
        ),
        FileTimeSection::Older => format!(
            "{:02} {} {}",
            modified_local.day(),
            month_abbreviation(modified_local),
            modified_local.year()
        ),
        FileTimeSection::UnknownDate => "—".to_string(),
    };

    FileTimePresentation { section, label }
}

fn resolve_local(
    system_time: SystemTime,
    resolve_offset: &impl Fn(OffsetDateTime) -> Option<UtcOffset>,
) -> Option<OffsetDateTime> {
    let utc = OffsetDateTime::from(system_time);
    resolve_offset(utc).map(|offset| utc.to_offset(offset))
}

fn unknown_date() -> FileTimePresentation {
    FileTimePresentation {
        section: FileTimeSection::UnknownDate,
        label: "—".to_string(),
    }
}

fn clock_label(datetime: OffsetDateTime) -> String {
    format!("{:02}:{:02}", datetime.hour(), datetime.minute())
}

fn month_abbreviation(datetime: OffsetDateTime) -> &'static str {
    match datetime.month() {
        time::Month::January => "Jan",
        time::Month::February => "Feb",
        time::Month::March => "Mar",
        time::Month::April => "Apr",
        time::Month::May => "May",
        time::Month::June => "Jun",
        time::Month::July => "Jul",
        time::Month::August => "Aug",
        time::Month::September => "Sep",
        time::Month::October => "Oct",
        time::Month::November => "Nov",
        time::Month::December => "Dec",
    }
}

fn weekday_abbreviation(datetime: OffsetDateTime) -> &'static str {
    match datetime.weekday() {
        time::Weekday::Monday => "Mon",
        time::Weekday::Tuesday => "Tue",
        time::Weekday::Wednesday => "Wed",
        time::Weekday::Thursday => "Thu",
        time::Weekday::Friday => "Fri",
        time::Weekday::Saturday => "Sat",
        time::Weekday::Sunday => "Sun",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        present_file_time_with_resolver, with_test_calendar_anchor, FileTimeSection,
        LocalCalendarAnchor,
    };
    use std::time::SystemTime;
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

    fn offset(hours: i8) -> UtcOffset {
        UtcOffset::from_hms(hours, 0, 0).expect("valid test offset")
    }

    fn local_time(
        year: i32,
        month: Month,
        day: u8,
        hour: u8,
        minute: u8,
        offset: UtcOffset,
    ) -> OffsetDateTime {
        let date = Date::from_calendar_date(year, month, day).expect("valid test date");
        let time = Time::from_hms(hour, minute, 0).expect("valid test time");
        PrimitiveDateTime::new(date, time).assume_offset(offset)
    }

    fn system_time(datetime: OffsetDateTime) -> SystemTime {
        datetime.into()
    }

    fn present_at(
        modified: Option<OffsetDateTime>,
        anchor: OffsetDateTime,
    ) -> super::FileTimePresentation {
        let local_offset = anchor.offset();
        present_file_time_with_resolver(
            modified.map(system_time),
            LocalCalendarAnchor::from_system_time(system_time(anchor)),
            |_| Some(local_offset),
        )
    }

    #[test]
    fn test_calendar_anchor_override_is_scoped_and_restores_nested_state() {
        let outer = LocalCalendarAnchor::from_system_time_at_offset(
            system_time(local_time(2026, Month::January, 10, 12, 0, UtcOffset::UTC)),
            UtcOffset::UTC,
        );
        let inner = LocalCalendarAnchor::from_system_time_at_offset(
            system_time(local_time(2026, Month::February, 20, 8, 0, UtcOffset::UTC)),
            UtcOffset::UTC,
        );

        with_test_calendar_anchor(outer, || {
            assert_eq!(LocalCalendarAnchor::now(), outer);
            with_test_calendar_anchor(inner, || {
                assert_eq!(LocalCalendarAnchor::now(), inner);
            });
            assert_eq!(LocalCalendarAnchor::now(), outer);
        });
        assert_ne!(LocalCalendarAnchor::now(), outer);
    }

    #[test]
    fn entry_time_sections_cover_future_today_and_yesterday_boundaries() {
        let local_offset = offset(1);
        let anchor = local_time(2026, Month::January, 10, 12, 0, local_offset);

        for (modified, section, label) in [
            (
                local_time(2026, Month::January, 11, 9, 15, local_offset),
                FileTimeSection::Future,
                "09:15",
            ),
            (
                local_time(2026, Month::January, 10, 0, 0, local_offset),
                FileTimeSection::Today,
                "00:00",
            ),
            (
                local_time(2026, Month::January, 10, 23, 59, local_offset),
                FileTimeSection::Today,
                "23:59",
            ),
            (
                local_time(2026, Month::January, 9, 23, 59, local_offset),
                FileTimeSection::Yesterday,
                "23:59",
            ),
        ] {
            let presentation = present_at(Some(modified), anchor);
            assert_eq!(presentation.section, section);
            assert_eq!(presentation.label, label);
            assert_eq!(presentation.section.label(), section.label());
        }
    }

    #[test]
    fn entry_time_previous_seven_days_are_inclusive_and_use_weekday_time() {
        let local_offset = offset(1);
        let anchor = local_time(2026, Month::January, 10, 12, 0, local_offset);

        for (day, label) in [(8, "Thu 08:05"), (3, "Sat 08:05")] {
            let presentation = present_at(
                Some(local_time(2026, Month::January, day, 8, 5, local_offset)),
                anchor,
            );
            assert_eq!(presentation.section, FileTimeSection::Previous7Days);
            assert_eq!(presentation.label, label);
        }
    }

    #[test]
    fn entry_time_older_labels_include_year_only_when_it_differs() {
        let local_offset = offset(1);
        let anchor = local_time(2026, Month::January, 10, 12, 0, local_offset);

        let same_year = present_at(
            Some(local_time(2026, Month::January, 2, 8, 5, local_offset)),
            anchor,
        );
        assert_eq!(same_year.section, FileTimeSection::Older);
        assert_eq!(same_year.label, "02 Jan");

        let previous_year = present_at(
            Some(local_time(2025, Month::December, 31, 8, 5, local_offset)),
            anchor,
        );
        assert_eq!(previous_year.section, FileTimeSection::Older);
        assert_eq!(previous_year.label, "31 Dec 2025");
    }

    #[test]
    fn entry_time_resolves_anchor_and_modified_offsets_independently() {
        let summer_offset = offset(2);
        let winter_offset = offset(1);
        let anchor = local_time(2026, Month::July, 1, 2, 30, summer_offset);
        let modified = local_time(2025, Month::December, 31, 23, 30, winter_offset);
        let modified_utc = modified.to_offset(UtcOffset::UTC);

        let presentation = present_file_time_with_resolver(
            Some(system_time(modified)),
            LocalCalendarAnchor::from_system_time(system_time(anchor)),
            |utc| {
                if utc == modified_utc {
                    Some(winter_offset)
                } else {
                    Some(summer_offset)
                }
            },
        );

        assert_eq!(presentation.section, FileTimeSection::Older);
        assert_eq!(presentation.label, "31 Dec 2025");
    }

    #[test]
    fn entry_time_unknown_date_covers_missing_time_and_offset_failure() {
        let local_offset = offset(1);
        let anchor = local_time(2026, Month::January, 10, 12, 0, local_offset);
        let anchor = LocalCalendarAnchor::from_system_time(system_time(anchor));

        let missing = present_file_time_with_resolver(None, anchor, |_| Some(local_offset));
        assert_eq!(missing.section, FileTimeSection::UnknownDate);
        assert_eq!(missing.section.label(), "Unknown Date");
        assert_eq!(missing.label, "—");

        let unresolved = present_file_time_with_resolver(
            Some(system_time(local_time(
                2026,
                Month::January,
                10,
                11,
                0,
                local_offset,
            ))),
            anchor,
            |_| None,
        );
        assert_eq!(unresolved.section, FileTimeSection::UnknownDate);
        assert_eq!(unresolved.label, "—");
    }
}
