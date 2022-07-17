//! Basic usage is:
//!
//! ```rust
//! for evn in XmlIter::from("<a min=\"0, 0\">Some Text</a>") {
//!     dbg!(evn);
//!     // do something with evn
//! }
//! ```
//!
//! You may want to keep a stack around to push values and modify it's attributes.

mod chars;
use chars::Chars;

/// Xml events returned from the [`XmlIter`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmlEvent<'a> {
    PushElement {
        name: &'a str,
    },
    PopElement {
        name: Option<&'a str>,
    },
    Attr {
        name: &'a str,
        value: Option<&'a str>,
    },
    Text {
        text: &'a str,
    },
}

/// Xml parser, it iterates over a stream of `chars` returning [`XmlEvent`]s
pub struct XmlIter<'a> {
    input: Chars<'a>,
    prop: bool,
}

impl<'a> From<&'a str> for XmlIter<'a> {
    fn from(input: &'a str) -> Self {
        Self {
            input: input.into(),
            prop: false,
        }
    }
}

impl<'a> Iterator for XmlIter<'a> {
    type Item = XmlEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.prop {
            self.element_events()
        } else {
            self.document_events()
        }
    }
}

impl<'a> XmlIter<'a> {
    #[inline]
    fn ignore_whitespace(&mut self) {
        while let Some(ch) = self.input.head() {
            if ch.is_whitespace() {
                self.input.next();
            } else {
                break;
            }
        }
    }

    fn ignore_comment(&mut self) {
        // expects input to be head = Some('!'), tail = "--"
        debug_assert!(self.input.head() == Some('!'));
        debug_assert!(self.input.tail().starts_with("--"));
        self.input.next(); // head = Some('-'), tail = "-..."
        self.input.next(); // head = Some('-'), tail = "..."

        loop {
            let rem = self.input.tail();
            if rem.starts_with("-->") {
                // head = Some(?), tail = "-->..."
                self.input.next(); // head = Some('-'), tail = "->..."
                self.input.next(); // head = Some('-'), tail = ">..."
                self.input.next(); // head = Some('>'), tail = "..."
                self.input.next(); // head = ?, tail = "..."
                break;
            } else {
                if self.input.next() == None {
                    break;
                }
            }
        }
    }

    fn document_events(&mut self) -> Option<XmlEvent<'a>> {
        loop {
            self.ignore_whitespace();

            match self.input.head() {
                Some('<') => {
                    // note: the tail doesn't contains the head
                    let rem = self.input.tail();
                    // consume '<'
                    self.input.next();
                    if rem.starts_with("/") {
                        // consume '/'
                        self.input.next();
                        return self.pop_element();
                    } else if rem.starts_with("!--") {
                        self.ignore_comment();
                        continue;
                    } else {
                        return self.push_element();
                    }
                }
                None => {
                    // end
                    return None;
                }
                _ => {
                    return self.push_text();
                }
            }
        }
    }

    fn push_text(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.input.cursor();
        while let Some(ch) = self.input.head() {
            if ch.is_whitespace() {
                return Some(XmlEvent::Text {
                    text: self.input.sub_str_from_cursor(cursor),
                });
            } else {
                self.input.next();
            }
        }
        None
    }

    fn push_element(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.input.cursor();
        while let Some(ch) = self.input.head() {
            if !ch.is_whitespace() && ch != '>' && ch != '/' {
                self.input.next();
            } else {
                let name = self.input.sub_str_from_cursor(cursor);
                if name.len() == 0 {
                    panic!("missing element name");
                }
                // subsequent calls to must return attributes from this property
                self.prop = true;
                return Some(XmlEvent::PushElement { name });
            }
        }
        None
    }

    fn pop_element(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.input.cursor();
        while let Some(ch) = self.input.head() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                self.input.next();
            } else if ch == '>' {
                let name = Some(self.input.sub_str_from_cursor(cursor));
                // consume '>'
                self.input.next();
                return Some(XmlEvent::PopElement { name });
            } else {
                panic!("unexpected char `{}`", ch);
            }
        }
        None
    }

    fn element_events(&mut self) -> Option<XmlEvent<'a>> {
        loop {
            self.ignore_whitespace();

            match self.input.head() {
                Some('<') => {
                    // consume '<'
                    self.input.next();
                    match self.input.head() {
                        Some('!') => self.ignore_comment(),
                        None => panic!("unexpected end of file"),
                        Some(ch) => panic!("unexpected char `{}`", ch),
                    }
                }
                Some('>') => {
                    // consume '>'
                    self.input.next();
                    // resume document level events
                    self.prop = false;
                    return self.document_events();
                }
                Some('/') => {
                    // consume '/'
                    self.input.next();
                    match self.input.head() {
                        Some('>') => {
                            // consume '>'
                            self.input.next();
                            // resume document level events
                            self.prop = false;
                            return Some(XmlEvent::PopElement { name: None });
                        }
                        None => {
                            panic!("unexpected end of file");
                        }
                        Some(ch) => {
                            panic!("unexpected char `{}`", ch);
                        }
                    }
                }
                None => {
                    // end
                    return None;
                }
                _ => {
                    return self.push_attr();
                }
            }
        }
    }

    fn push_attr(&mut self) -> Option<XmlEvent<'a>> {
        // attribute name
        let name;
        let cursor = self.input.cursor();
        loop {
            if let Some(ch) = self.input.head() {
                if !ch.is_whitespace() && ch != '=' {
                    self.input.next();
                } else {
                    name = self.input.sub_str_from_cursor(cursor);
                    if name.len() == 0 {
                        panic!("missing attribute name");
                    }
                    break;
                }
            } else {
                panic!("unexpected end of file");
            }
        }

        self.ignore_whitespace();

        if self.input.head() != Some('=') {
            // attribute has no value
            return Some(XmlEvent::Attr { name, value: None });
        }

        // consume '='
        self.input.next();
        self.ignore_whitespace();

        // expect and consume '\"'
        match self.input.head() {
            Some('\"') => {
                self.input.next();
            }
            None => panic!("unexpected end of file"),
            Some(ch) => panic!("unexpected char `{}`", ch),
        }

        // attribute value
        let value;
        let cursor = self.input.cursor();
        loop {
            match self.input.head() {
                Some('\"') => {
                    value = Some(self.input.sub_str_from_cursor(cursor));
                    self.input.next();
                    break;
                }
                Some('\\') => {
                    // consume '\\'
                    self.input.next();
                    // ignore scaped char, any escaped utf8 chars in the format '\uXXXX' should be covered
                    self.input.next();
                }
                None => {
                    panic!("unexpected end of file");
                }
                _ => {
                    // keep reading string
                    self.input.next();
                }
            }
        }

        return Some(XmlEvent::Attr { name, value });
    }
}

#[cfg(test)]
mod tests {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    struct Allocator;

    static ALLOCATIONS_COUNT: AtomicUsize = AtomicUsize::new(0);

    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            ALLOCATIONS_COUNT.fetch_add(1, Ordering::Relaxed);
            System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            System.dealloc(ptr, layout)
        }
    }

    #[global_allocator]
    static GLOBAL: Allocator = Allocator;

    #[test]
    fn alloc() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        let allocation = Box::new('x');

        assert_eq!(
            ALLOCATIONS_COUNT.load(Ordering::Relaxed),
            1,
            "didn't count allocation"
        );
        assert!(*allocation == 'x');
    }

    fn cmp<'a, 'b>(
        mut a: impl Iterator<Item = XmlEvent<'a>>,
        mut b: impl Iterator<Item = XmlEvent<'b>>,
    ) {
        loop {
            let a = a.next();
            let b = b.next();
            if a != b {
                panic!("{:?} != {:?}", a, b);
            }
            if a.is_none() {
                break;
            }
        }
    }

    #[test]
    fn elements() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        cmp(
            XmlIter::from("<r></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r><a><b></b></a></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PushElement { name: "a" },
                XmlEvent::PushElement { name: "b" },
                XmlEvent::PopElement { name: Some("b") },
                XmlEvent::PopElement { name: Some("a") },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r/>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PopElement { name: None },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r />"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PopElement { name: None },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }

    #[test]
    fn comments() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        cmp(XmlIter::from("<!--<r></r>-->"), [].iter().copied());
        cmp(XmlIter::from("<!--<r></r>"), [].iter().copied());

        cmp(
            XmlIter::from("<r> <!-- text --> </r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r <!-- min=\"0, 0\" --> ></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }

    #[test]
    fn attributes() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        cmp(
            XmlIter::from("<r min=\"0, 0\"></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::Attr {
                    name: "min",
                    value: Some("0, 0"),
                },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r toggle color=\"#fff\"></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::Attr {
                    name: "toggle",
                    value: None,
                },
                XmlEvent::Attr {
                    name: "color",
                    value: Some("#fff"),
                },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from(r#"<r text="\"escaped\" sequence \u0041"></r>"#),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::Attr {
                    name: "text",
                    value: Some("\\\"escaped\\\" sequence \\u0041"),
                },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }

    #[test]
    fn text() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        cmp(
            XmlIter::from("<a>  some text  </a>"),
            [
                XmlEvent::PushElement { name: "a" },
                XmlEvent::Text { text: "some" },
                XmlEvent::Text { text: "text" },
                XmlEvent::PopElement { name: Some("a") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<a>  some <!-- not --> text  </a>"),
            [
                XmlEvent::PushElement { name: "a" },
                XmlEvent::Text { text: "some" },
                XmlEvent::Text { text: "text" },
                XmlEvent::PopElement { name: Some("a") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("  text\n only  "),
            [
                XmlEvent::Text { text: "text" },
                XmlEvent::Text { text: "only" },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }
}
