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
}

impl LocalCalendarAnchor {
    pub(crate) fn now() -> Self {
        Self {
            now: SystemTime::now(),
        }
    }

    pub(crate) const fn from_system_time(now: SystemTime) -> Self {
        Self { now }
    }
}

pub(crate) fn present_file_time(
    modified: Option<SystemTime>,
    anchor: LocalCalendarAnchor,
) -> FileTimePresentation {
    present_file_time_with_resolver(modified, anchor, |utc| UtcOffset::local_offset_at(utc).ok())
}

fn present_file_time_with_resolver(
    _modified: Option<SystemTime>,
    _anchor: LocalCalendarAnchor,
    _resolve_offset: impl Fn(OffsetDateTime) -> Option<UtcOffset>,
) -> FileTimePresentation {
    FileTimePresentation {
        section: FileTimeSection::UnknownDate,
        label: "—".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{present_file_time_with_resolver, FileTimeSection, LocalCalendarAnchor};
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
