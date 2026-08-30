//! A clock a bar section can draw.
//!
//! The format is parsed once, when the config is read, into a list of pieces.
//! Drawing then walks that list — which is what keeps `render` free of both
//! parsing and of `now()`. The reading arrives already taken, exactly the way a
//! resource sample does, and for the same reason: a widget that reached for a
//! clock inside `render` would look correct on screen and resend its cells on
//! every frame.

use std::time::Duration;

use time::OffsetDateTime;

/// One field a clock format can name, and the character that names it.
///
/// A table rather than a `match`, for the reason the bar's grammar keeps tables
/// (D70-7): a `match` can be asked whether it knows a spelling but never what
/// spellings it knows, and this set has to be printable — by `herdr shell
/// spec`, by the refusal message, and by the guide's own gate.
pub(crate) struct ClockField {
    /// The character after `%`.
    pub spec: char,
    pub kind: ClockFieldKind,
    /// What it renders as, published beside the name.
    ///
    /// A real rendering rather than a description, and all eleven are of one
    /// moment — 2026-08-17, a Monday, at 13:05:09 — so the table reads as a
    /// single clock taken apart. A prose description would hide the case people
    /// get wrong: `%I` at that hour is `01`, and at midnight it is `12`.
    pub example: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClockFieldKind {
    Hour24,
    Hour12,
    Minute,
    Second,
    Meridiem,
    Day,
    MonthNumber,
    MonthName,
    Weekday,
    Year4,
    Year2,
}

/// Every field this build understands, in the order a refusal offers them.
pub(crate) const CLOCK_FIELDS: &[ClockField] = &[
    ClockField {
        spec: 'H',
        kind: ClockFieldKind::Hour24,
        example: "13",
    },
    ClockField {
        spec: 'I',
        kind: ClockFieldKind::Hour12,
        example: "01",
    },
    ClockField {
        spec: 'M',
        kind: ClockFieldKind::Minute,
        example: "05",
    },
    ClockField {
        spec: 'S',
        kind: ClockFieldKind::Second,
        example: "09",
    },
    ClockField {
        spec: 'p',
        kind: ClockFieldKind::Meridiem,
        example: "PM",
    },
    ClockField {
        spec: 'a',
        kind: ClockFieldKind::Weekday,
        example: "Mon",
    },
    ClockField {
        spec: 'd',
        kind: ClockFieldKind::Day,
        example: "17",
    },
    ClockField {
        spec: 'b',
        kind: ClockFieldKind::MonthName,
        example: "Aug",
    },
    ClockField {
        spec: 'm',
        kind: ClockFieldKind::MonthNumber,
        example: "08",
    },
    ClockField {
        spec: 'Y',
        kind: ClockFieldKind::Year4,
        example: "2026",
    },
    ClockField {
        spec: 'y',
        kind: ClockFieldKind::Year2,
        example: "26",
    },
];

/// Every field spelling, as a config writes it.
pub(crate) fn clock_field_names() -> Vec<String> {
    CLOCK_FIELDS
        .iter()
        .map(|field| format!("%{}", field.spec))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ClockPiece {
    Literal(String),
    Field(ClockFieldKind),
}

/// A parsed clock format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClockFormat {
    pieces: Vec<ClockPiece>,
}

impl Default for ClockFormat {
    /// `%H:%M` — the shape a bar clock has when nobody said otherwise.
    ///
    /// Deliberately without seconds. A default that ticked every second would
    /// make the cheapest possible clock the one nobody chose, and the person
    /// who wants seconds can ask for them by name.
    fn default() -> Self {
        Self::parse("%H:%M").unwrap_or(Self { pieces: Vec::new() })
    }
}

impl ClockFormat {
    /// Parse a format, or say which field spelling was not understood.
    ///
    /// `%%` is a literal percent. An unknown `%Q` is refused rather than passed
    /// through: somebody seeing `%Q` on their bar cannot tell a spelling this
    /// build lacks from one it has and drew wrongly. Refusing costs nothing
    /// here — the key is new, so no file that loads today contains one.
    pub(crate) fn parse(raw: &str) -> Result<Self, char> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut glyphs = raw.chars();
        while let Some(glyph) = glyphs.next() {
            if glyph != '%' {
                literal.push(glyph);
                continue;
            }
            // A trailing `%` names no field at all. Reported as `%` itself,
            // which is what the person wrote and what they have to find.
            let Some(spec) = glyphs.next() else {
                return Err('%');
            };
            if spec == '%' {
                literal.push('%');
                continue;
            }
            let Some(known) = CLOCK_FIELDS.iter().find(|field| field.spec == spec) else {
                return Err(spec);
            };
            if !literal.is_empty() {
                pieces.push(ClockPiece::Literal(std::mem::take(&mut literal)));
            }
            pieces.push(ClockPiece::Field(known.kind));
        }
        if !literal.is_empty() {
            pieces.push(ClockPiece::Literal(literal));
        }
        Ok(Self { pieces })
    }

    /// How often the text this format renders can change.
    ///
    /// The whole cost argument for the widget, and the reason it is derived
    /// rather than configured beside the format: the format already says what
    /// resolution the person asked for, and a second key saying it again would
    /// only raise the question of which one wins when they disagree.
    ///
    /// A clock showing seconds has to be redrawn every second. One showing
    /// `%H:%M` changes sixty times less often, and waking every second to
    /// render the same string would push those cells through the frame diff for
    /// nothing — which is the cost this product does not pay while nobody is
    /// looking.
    pub(crate) fn resolution(&self) -> Duration {
        let shows_seconds = self
            .pieces
            .iter()
            .any(|piece| matches!(piece, ClockPiece::Field(ClockFieldKind::Second)));
        Duration::from_secs(if shows_seconds { 1 } else { 60 })
    }

    /// Whether two readings show different faces at this resolution.
    ///
    /// The comparison must match what `render` can print: a minute clock's
    /// face is identical across the fifty-nine seconds inside a minute, and
    /// calling each of those a change hands the whole surface a frame per
    /// second for a string that did not move — the exact cost `resolution`
    /// exists to avoid. A missing reading on either side is a change, because
    /// a face appearing or vanishing is one. TP-CLOCK-13
    pub(crate) fn faces_differ(
        previous: Option<OffsetDateTime>,
        current: Option<OffsetDateTime>,
        resolution: Duration,
    ) -> bool {
        let shows_seconds = resolution < Duration::from_secs(60);
        let face = |at: OffsetDateTime| {
            (
                at.hour(),
                at.minute(),
                if shows_seconds {
                    Some(at.second())
                } else {
                    None
                },
            )
        };
        previous.map(face) != current.map(face)
    }

    pub(crate) fn render(&self, at: OffsetDateTime) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                ClockPiece::Literal(text) => out.push_str(text),
                ClockPiece::Field(kind) => out.push_str(&render_field(*kind, at)),
            }
        }
        out
    }
}

fn render_field(kind: ClockFieldKind, at: OffsetDateTime) -> String {
    match kind {
        ClockFieldKind::Hour24 => format!("{:02}", at.hour()),
        // Midnight is 12 and noon is 12. `hour % 12` alone is the classic way
        // to put `00` on a twelve-hour clock, which no twelve-hour clock shows.
        ClockFieldKind::Hour12 => {
            let hour = at.hour() % 12;
            format!("{:02}", if hour == 0 { 12 } else { hour })
        }
        ClockFieldKind::Minute => format!("{:02}", at.minute()),
        ClockFieldKind::Second => format!("{:02}", at.second()),
        ClockFieldKind::Meridiem => if at.hour() < 12 { "AM" } else { "PM" }.to_string(),
        ClockFieldKind::Day => format!("{:02}", at.day()),
        ClockFieldKind::MonthNumber => format!("{:02}", u8::from(at.month())),
        // Borrowed rather than copied: the file manager already spells these
        // out, and two tables of month names are two places for a translation
        // or an abbreviation style to drift apart.
        ClockFieldKind::MonthName => crate::fm::entry_time::month_abbreviation(at).to_string(),
        ClockFieldKind::Weekday => crate::fm::entry_time::weekday_abbreviation(at).to_string(),
        ClockFieldKind::Year4 => format!("{}", at.year()),
        ClockFieldKind::Year2 => format!("{:02}", at.year().rem_euclid(100)),
    }
}

/// The wall clock, in the local zone, or `None` when the zone cannot be read.
///
/// `None` rather than a fallback to UTC: a clock quietly showing another
/// country's time is worse than a clock showing nothing, because nothing about
/// it looks wrong. The same rule a metric that cannot be read follows.
pub(crate) fn local_now() -> Option<OffsetDateTime> {
    OffsetDateTime::now_local().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, PrimitiveDateTime, Time, UtcOffset};

    // TP-CLOCK-13: a minute clock's face is one string across a minute — a
    // new second inside it is not a change, and a seconds clock still sees it.
    #[test]
    fn a_minute_face_ignores_a_new_second() {
        let earlier = anchor();
        let later = earlier
            .replace_second(earlier.second() + 1)
            .expect("valid second");
        assert!(!ClockFormat::faces_differ(
            Some(earlier),
            Some(later),
            Duration::from_secs(60)
        ));
        assert!(
            ClockFormat::faces_differ(Some(earlier), Some(later), Duration::from_secs(1)),
            "a seconds clock still sees the new second"
        );
    }

    // TP-CLOCK-13: the minute moving is what a minute face calls a change.
    #[test]
    fn a_minute_face_changes_when_the_minute_does() {
        let earlier = anchor();
        let later = earlier
            .replace_minute(earlier.minute() + 1)
            .expect("valid minute");
        assert!(ClockFormat::faces_differ(
            Some(earlier),
            Some(later),
            Duration::from_secs(60)
        ));
    }

    // TP-CLOCK-13: a face appearing or vanishing is a change at any pace.
    #[test]
    fn a_face_appearing_or_vanishing_is_a_change() {
        let now = anchor();
        for resolution in [Duration::from_secs(1), Duration::from_secs(60)] {
            assert!(ClockFormat::faces_differ(None, Some(now), resolution));
            assert!(ClockFormat::faces_differ(Some(now), None, resolution));
            assert!(!ClockFormat::faces_differ(None, None, resolution));
        }
    }

    /// 2026-08-17 13:05:09 — a Monday, in the afternoon, so `%I`/`%p` and the
    /// two-digit padding of every other field are all exercised by one moment.
    fn anchor() -> OffsetDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::August, 17).expect("a real date"),
            Time::from_hms(13, 5, 9).expect("a real time"),
        )
        .assume_offset(UtcOffset::UTC)
    }

    /// Every published field renders the example published beside it.
    ///
    /// The table carries an example so `herdr shell spec` can show one, and an
    /// example nobody checks is a sentence that ages the first time a field's
    /// padding changes. Reading it back through the real renderer makes the
    /// published table a claim this build has to keep.
    // TP-CLOCK-01: every clock field renders the example the table publishes.
    #[test]
    fn every_clock_field_renders_the_example_it_publishes() {
        for field in CLOCK_FIELDS {
            let format = ClockFormat::parse(&format!("%{}", field.spec)).expect("a listed field");
            assert_eq!(
                format.render(anchor()),
                field.example,
                "the table says %{} renders as {:?}",
                field.spec,
                field.example
            );
        }
    }

    /// Midnight is twelve o'clock, not zero.
    // TP-CLOCK-02: a twelve-hour clock never shows `00`.
    #[test]
    fn a_twelve_hour_clock_shows_twelve_at_midnight_and_noon() {
        let format = ClockFormat::parse("%I%p").expect("a listed field");
        let at = |hour| {
            PrimitiveDateTime::new(
                Date::from_calendar_date(2026, Month::August, 17).expect("a real date"),
                Time::from_hms(hour, 0, 0).expect("a real time"),
            )
            .assume_offset(UtcOffset::UTC)
        };

        assert_eq!(format.render(at(0)), "12AM", "midnight is twelve, not zero");
        assert_eq!(format.render(at(12)), "12PM", "noon is twelve as well");
        assert_eq!(format.render(at(13)), "01PM");
        assert_eq!(format.render(at(11)), "11AM");
    }

    /// Literal text survives, and `%%` is one percent.
    // TP-CLOCK-03: everything that is not a field is carried through verbatim.
    #[test]
    fn literal_text_between_fields_is_carried_through() {
        let format = ClockFormat::parse("[%H:%M] %d %b — 100%%").expect("all fields are listed");
        assert_eq!(format.render(anchor()), "[13:05] 17 Aug — 100%");
    }

    /// A field this build does not know is refused, by the character written.
    // TP-CLOCK-04: an unknown field is refused rather than drawn as itself.
    #[test]
    fn a_field_this_build_does_not_know_is_refused_by_name() {
        assert_eq!(ClockFormat::parse("%H:%Q").unwrap_err(), 'Q');
        assert_eq!(
            ClockFormat::parse("%H:%").unwrap_err(),
            '%',
            "a trailing percent names no field, and is reported as what was written"
        );
        // Case matters, the same way it does for every other name in the bar
        // grammar: a near-miss is a typo, not another spelling.
        assert_eq!(ClockFormat::parse("%h").unwrap_err(), 'h');
    }

    /// The resolution comes from the format, and only seconds make it fast.
    ///
    /// The control half is what makes this a cost gate rather than a spelling
    /// check: a format holding every other field must still be a once-a-minute
    /// clock, or the widget wakes sixty times more often than it has anything
    /// new to say.
    // TP-CLOCK-05: only a format showing seconds refreshes every second.
    #[test]
    fn only_a_format_showing_seconds_refreshes_every_second() {
        assert_eq!(
            ClockFormat::parse("%H:%M:%S").expect("listed").resolution(),
            Duration::from_secs(1)
        );
        assert_eq!(
            ClockFormat::default().resolution(),
            Duration::from_secs(60),
            "the default must not be the expensive one"
        );

        let everything_else = CLOCK_FIELDS
            .iter()
            .filter(|field| field.kind != ClockFieldKind::Second)
            .map(|field| format!("%{}", field.spec))
            .collect::<String>();
        assert_eq!(
            ClockFormat::parse(&everything_else)
                .expect("listed")
                .resolution(),
            Duration::from_secs(60),
            "every field except seconds must leave the clock on the slow tick"
        );
    }

    /// The same minute renders the same text, whatever second it is read at.
    ///
    /// This is what makes the slow tick safe: a clock woken at :00 and again at
    /// :59 of one minute has nothing new to draw, so the frame diff sends
    /// nothing. Without it, a once-a-minute wakeup could still repaint.
    // TP-CLOCK-06: a minute-resolution clock renders one string per minute.
    #[test]
    fn a_clock_without_seconds_renders_the_same_text_all_minute() {
        let format = ClockFormat::default();
        let at = |second| {
            PrimitiveDateTime::new(
                Date::from_calendar_date(2026, Month::August, 17).expect("a real date"),
                Time::from_hms(13, 5, second).expect("a real time"),
            )
            .assume_offset(UtcOffset::UTC)
        };

        let first = format.render(at(0));
        for second in [1, 30, 59] {
            assert_eq!(
                format.render(at(second)),
                first,
                "second {second} of the same minute must draw the same text"
            );
        }
        assert_ne!(
            format.render(at(0)),
            ClockFormat::parse("%H:%M:%S")
                .expect("listed")
                .render(at(0)),
            "control: a format that does show seconds must render differently, \
             or the assertion above would hold for a renderer that ignores time"
        );
    }
}
