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

#![cfg_attr(not(test), no_std)]

mod fallback;
mod sse2;

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
    Comment {
        text: &'a str,
    },
}

#[inline(always)]
const unsafe fn str_from_raw_parts<'a>(ptr: *const u8, len: usize) -> &'a str {
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
}

#[inline(always)]
const unsafe fn str_from_range<'a>(ptr: *const u8, ptr_end: *const u8) -> &'a str {
    str_from_raw_parts(ptr, ptr_end.offset_from(ptr) as usize)
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
