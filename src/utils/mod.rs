#![allow(unused)]
use std::borrow::Cow;

pub trait MaybeReplaceExt<'a> {
    fn maybe_replace(self, needle: &str, replacement: &str) -> Cow<'a, str>;
    fn maybe_replace_closure<F>(self, needle: &str, replacement: F) -> Cow<'a, str>
    where
        F: FnOnce() -> String;
}

impl<'a> MaybeReplaceExt<'a> for &'a str {
    fn maybe_replace(self, needle: &str, replacement: &str) -> Cow<'a, str> {
        // Assumes that searching twice is better than unconditionally allocating
        if self.contains(needle) {
            self.replace(needle, replacement).into()
        } else {
            self.into()
        }
    }

    fn maybe_replace_closure<F>(self, needle: &str, replacement: F) -> Cow<'a, str>
    where
        F: FnOnce() -> String,
    {
        if self.contains(needle) {
            self.replace(needle, &replacement()).into()
        } else {
            self.into()
        }
    }
}

impl<'a> MaybeReplaceExt<'a> for Cow<'a, str> {
    fn maybe_replace(self, needle: &str, replacement: &str) -> Cow<'a, str> {
        // Assumes that searching twice is better than unconditionally allocating
        if self.contains(needle) {
            self.replace(needle, replacement).into()
        } else {
            self
        }
    }

    fn maybe_replace_closure<F>(self, needle: &str, replacement: F) -> Cow<'a, str>
    where
        F: FnOnce() -> String,
    {
        if self.contains(needle) {
            self.replace(needle, &replacement()).into()
        } else {
            self
        }
    }
}

/// Replaces variables in the template URL with values from the CSV record.
/// Variables in the template should be in the format <%COLUMN_NAME%>.
pub fn replace_vars_from_csv(
    template: &str,
    headers: &csv::StringRecord,
    record: &csv::StringRecord,
) -> String {
    let mut url = Cow::from(template);
    for (i, header) in headers.iter().enumerate() {
        let var = format!("<%{header}%>");
        if let Some(value) = record.get(i) {
            url = url.maybe_replace(&var, value);
        }
    }
    url.into()
}
