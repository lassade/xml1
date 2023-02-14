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
//!
//! Less than sign `'<'` must be escaped during texts sequeces

/// Points to a valid UTF8 character inside a [`str`], used to take sub strings
#[derive(Copy, Clone)]
struct Cursor {
    ptr: *const u8,
}

struct Text<'a> {
    text: &'a str,
}

impl<'a> Text<'a> {
    const fn len(&mut self) -> usize {
        self.text.len()
    }

    fn next(&mut self) -> bool {
        if self.text.len() == 0 {
            return false;
        }

        // safety: assumes that text is a valid utf8 string1,
        unsafe {
            let offset = (*self.text.as_ptr()).leading_ones() as usize;
            self.text = core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                self.text.as_ptr().add(offset),
                self.text.len() - offset,
            ));
        }

        true
    }

    fn next_cond(&mut self, v: &str) -> bool {
        if self.text.as_bytes().starts_with(v.as_bytes()) {
            unsafe {
                let offset = v.len();
                self.text = core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    self.text.as_ptr().add(offset),
                    self.text.len() - offset,
                ));
            }
            true
        } else {
            false
        }
    }

    fn rtrim(&mut self) {
        const ASCII: [u8; 128] = [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        ];
        #[inline(always)]
        pub unsafe fn ascii_lookup(x: u8) -> bool {
            *ASCII.get_unchecked(x as usize) == 0
        }

        static UNICODE: [u8; 256] = [
            2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        #[inline(always)]
        pub unsafe fn unicode_lookup(x: u8, y: u8) -> bool {
            match y {
                0 => UNICODE.get_unchecked(x as usize) & 1 != 0,
                22 => x == 0x80 && y == 0x16,
                32 => UNICODE.get_unchecked(x as usize) & 2 != 0,
                48 => x == 0x00 && y == 0x30,
                _ => false,
            }
        }

        let ptr = self.text.as_ptr();
        unsafe {
            let ptr_end = ptr.add(self.text.len());
            loop {
                if ptr >= ptr_end {
                    break;
                }

                let x = *ptr;
                if x > 127 {
                    // process the unicode char
                    let y = *ptr.add(1);
                    if unicode_lookup(x, y) {
                        let offset = x.leading_ones() as usize;
                        ptr = ptr.add(offset);
                        continue;
                    } else {
                        break;
                    }
                } else {
                    // ascii char
                    if !ascii_lookup(x) {
                        break;
                    }
                }

                ptr = ptr.add(1);
            }

            self.text = core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                ptr,
                ptr_end.offset_from(ptr) as usize, // todo: use sub_ptr once it get stable
            ));
        }
    }

    #[inline(always)]
    pub fn cursor(&self) -> Cursor {
        Cursor {
            ptr: self.text.as_ptr(),
        }
    }

    /// SAFETY: Must the cursor must be from the same text
    #[inline(always)]
    pub unsafe fn sub_str_from_cursor(&self, cursor: Cursor) -> &'a str {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            cursor.ptr,
            self.text.as_ptr().offset_from(cursor.ptr) as _,
        ))
    }
}

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
    text: Text<'a>,
    prop: bool,
}

impl<'a> From<&'a str> for XmlIter<'a> {
    fn from(input: &'a str) -> Self {
        Self {
            text: Text { text: input },
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
    fn ignore_comment(&mut self) {
        loop {
            if self.text.next_cond("-->") {
                break;
            } else {
                if !self.text.next() {
                    break;
                }
            }
        }
    }

    fn document_events(&mut self) -> Option<XmlEvent<'a>> {
        loop {
            self.text.rtrim();

            if self.text.len() == 0 {
                // end
                return None;
            } else if self.text.next_cond("</") {
                return self.pop_element();
            } else if self.text.next_cond("<!--") {
                self.ignore_comment();
                continue;
            } else if self.text.next_cond("<") {
                return self.push_element();
            } else {
                return self.push_text();
            }
        }
    }

    fn push_text(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.text.cursor();
        loop {
            if self.text.next_cond("\n") || self.text.next_cond("<") {
                break;
            }
        }
        Some(XmlEvent::Text {
            // safety: cursor is from the same str
            text: unsafe { self.text.sub_str_from_cursor(cursor).trim_end() },
        })
    }

    fn push_element(&mut self) -> Option<XmlEvent<'a>> {
        // search for "/>" while returning the text in between

        let cursor = self.text.cursor();
        while let Some(ch) = self.text.head() {
            if !ch.is_whitespace() && ch != '>' && ch != '/' {
                self.text.next();
            } else {
                let name = self.text.sub_str_from_cursor(cursor);
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
        let cursor = self.text.cursor();
        while let Some(ch) = self.text.head() {
            if !ch.is_whitespace() && ch != '>' {
                self.text.next();
            } else {
                let name = Some(self.text.sub_str_from_cursor(cursor));

                self.text.rtrim();
                match self.text.head() {
                    Some('>') => {
                        // consume '>'
                        self.text.next();
                    }
                    Some(ch) => {
                        panic!("unexpected char `{}` (\\u{:X})", ch, ch as u32);
                    }
                    None => panic!("unexpected end of file"),
                }

                return Some(XmlEvent::PopElement { name });
            }
        }
        None
    }

    fn element_events(&mut self) -> Option<XmlEvent<'a>> {
        loop {
            self.text.rtrim();

            match self.text.head() {
                Some('<') => {
                    // consume '<'
                    self.text.next();
                    match self.text.head() {
                        Some('!') => self.ignore_comment(),
                        None => panic!("unexpected end of file"),
                        Some(ch) => panic!("unexpected char `{}` (\\u{:X})", ch, ch as u32),
                    }
                }
                Some('>') => {
                    // consume '>'
                    self.text.next();
                    // resume document level events
                    self.prop = false;
                    return self.document_events();
                }
                Some('/') => {
                    // consume '/'
                    self.text.next();
                    match self.text.head() {
                        Some('>') => {
                            // consume '>'
                            self.text.next();
                            // resume document level events
                            self.prop = false;
                            return Some(XmlEvent::PopElement { name: None });
                        }
                        None => {
                            panic!("unexpected end of file");
                        }
                        Some(ch) => {
                            panic!("unexpected char `{}` (\\u{:X})", ch, ch as u32);
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
        let cursor = self.text.cursor();
        loop {
            if let Some(ch) = self.text.head() {
                if !ch.is_whitespace() && ch != '=' && ch != '>' && ch != '/' {
                    self.text.next();
                } else {
                    name = self.text.sub_str_from_cursor(cursor);
                    if name.len() == 0 {
                        panic!("missing attribute name");
                    }
                    break;
                }
            } else {
                panic!("unexpected end of file");
            }
        }

        self.text.rtrim();

        if self.text.head() != Some('=') {
            // attribute has no value
            return Some(XmlEvent::Attr { name, value: None });
        }

        // consume '='
        self.text.next();
        self.text.rtrim();

        // expect and consume '\"'
        match self.text.head() {
            Some('\"') => {
                self.text.next();
            }
            None => panic!("unexpected end of file"),
            Some(ch) => panic!("unexpected char `{}` (\\u{:X})", ch, ch as u32),
        }

        // attribute value
        let value;
        let cursor = self.text.cursor();
        loop {
            match self.text.head() {
                Some('\"') => {
                    value = Some(self.text.sub_str_from_cursor(cursor));
                    self.text.next();
                    break;
                }
                Some('\\') => {
                    // consume '\\'
                    self.text.next();
                    // ignore scaped char, any escaped utf8 chars in the format '\uXXXX' should be covered
                    self.text.next();
                }
                None => {
                    panic!("unexpected end of file");
                }
                _ => {
                    // keep reading string
                    self.text.next();
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
    fn is_counting_allocations() {
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
            XmlIter::from("<r clip></r>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::Attr {
                    name: "clip",
                    value: None,
                },
                XmlEvent::PopElement { name: Some("r") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<r clip/>"),
            [
                XmlEvent::PushElement { name: "r" },
                XmlEvent::Attr {
                    name: "clip",
                    value: None,
                },
                XmlEvent::PopElement { name: None },
            ]
            .iter()
            .copied(),
        );

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
                XmlEvent::Text { text: "some text" },
                XmlEvent::PopElement { name: Some("a") },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from("<a>some text</a>"),
            [
                XmlEvent::PushElement { name: "a" },
                XmlEvent::Text { text: "some text" },
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

        // doesnt support embedding '<' or '>' during the texts
        cmp(
            XmlIter::from("<a>20 &lt; 30</a>"),
            [
                XmlEvent::PushElement { name: "a" },
                XmlEvent::Text { text: "20 &lt; 30" },
                XmlEvent::PopElement { name: Some("a") },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }

    #[test]
    fn multiline_text() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        // big text chunck
        cmp(
            XmlIter::from(
                r#"<a color="rgb(0, 0, 0)">
                    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque rhoncus dui in leo mollis,
                    eleifend auctor neque gravida. Mauris quis tortor eget quam porttitor vulputate.
                    Ut cursus quam vitae turpis bibendum congue. Orci varius natoque penatibus et magnis dis parturient montes,
                    nascetur ridiculus mus. Ut tincidunt eu arcu eu dapibus. Nunc non urna orci. Quisque sit amet nisi viverra,
                    malesuada lacus id, congue neque.
                </a>"#,
            ),
            [
                XmlEvent::PushElement { name: "a" },
                XmlEvent::Attr { name: "color", value: Some("rgb(0, 0, 0)") },
                XmlEvent::Text { text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Pellentesque rhoncus dui in leo mollis," },
                XmlEvent::Text { text: "eleifend auctor neque gravida. Mauris quis tortor eget quam porttitor vulputate." },
                XmlEvent::Text { text: "Ut cursus quam vitae turpis bibendum congue. Orci varius natoque penatibus et magnis dis parturient montes," },
                XmlEvent::Text { text: "nascetur ridiculus mus. Ut tincidunt eu arcu eu dapibus. Nunc non urna orci. Quisque sit amet nisi viverra," },
                XmlEvent::Text { text: "malesuada lacus id, congue neque." },
                XmlEvent::PopElement { name: Some("a") },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }

    #[test]
    fn full_utf8_support() {
        // reset allocations
        ALLOCATIONS_COUNT.store(0, Ordering::Relaxed);

        cmp(
            XmlIter::from(r#"<サイボーグ 難易度="難しい" ></サイボーグ>"#),
            [
                XmlEvent::PushElement {
                    name: "サイボーグ"
                },
                XmlEvent::Attr {
                    name: "難易度",
                    value: Some("難しい"),
                },
                XmlEvent::PopElement {
                    name: Some("サイボーグ"),
                },
            ]
            .iter()
            .copied(),
        );

        // \u{200F} unsures vscode will draw the string right
        cmp(
            XmlIter::from(r#"‏ <سايبورغ الصعوبة="صعب"> </سايبورغ>"#),
            [
                XmlEvent::PushElement {
                    name: "سايبورغ"
                },
                XmlEvent::Attr {
                    name: "الصعوبة",
                    value: Some("صعب"),
                },
                XmlEvent::PopElement {
                    name: Some("سايبورغ"),
                },
            ]
            .iter()
            .copied(),
        );

        cmp(
            XmlIter::from(r#"<☕ ⚪="⚽" ></☕>"#),
            [
                XmlEvent::PushElement { name: "☕" },
                XmlEvent::Attr {
                    name: "⚪",
                    value: Some("⚽"),
                },
                XmlEvent::PopElement { name: Some("☕") },
            ]
            .iter()
            .copied(),
        );

        assert_eq!(ALLOCATIONS_COUNT.load(Ordering::Relaxed), 0, "allocated");
    }
}
