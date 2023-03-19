#![allow(unused)]

#[cfg(target_arch = "x86")]
use core::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

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
    EOF,
}

// is more strict about the input
// the only whitespace supported is the original ASCII 0x20 or U+0020

// main goal is to find the folling tokens '<', '=', ' ', '/', '?', '>'

#[inline(always)]
const unsafe fn str_from_raw_parts<'a>(ptr: *const u8, len: usize) -> &'a str {
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
}

#[inline(always)]
unsafe fn ignore_spaces(chunk: __m128i) -> i32 {
    !_mm_movemask_epi8(_mm_or_si128(
        _mm_or_si128(
            _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20
            _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\n' as i8)), // 0x0A
        ),
        _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\t' as i8)),
    ))
}

pub fn parse_xml<'a>(input: &'a str, mut f: impl FnMut(XmlEvent<'a>)) {
    let data = input.as_bytes();

    let mut ptr = data.as_ptr();
    let ptr_end = unsafe { ptr.add(data.len()) };

    loop {
        unsafe {
            if ptr >= ptr_end {
                (f)(XmlEvent::EOF);
                return;
            }

            let chunk = _mm_loadu_si128(ptr as *const _); // 6

            // ignore ' ', '\n' or '\t'
            let space_mask = ignore_spaces(chunk); // 6
            if space_mask != 0 {
                let space_offset = space_mask.trailing_zeros() as usize;

                // out of bounds check
                ptr = ptr.add(space_offset);
                if ptr >= ptr_end {
                    (f)(XmlEvent::EOF);
                    return;
                }

                let token = *ptr;
                if token == b'<' {
                    // move next
                    ptr = ptr.add(1);
                    let mut offset = 0;
                    let rem = ptr_end.offset_from(ptr) as usize;

                    // begin element
                    loop {
                        // out of bounds check
                        let ptr_next = ptr.add(offset);
                        if ptr_next >= ptr_end {
                            // use the remainig text to rise the event
                            offset = rem;
                            break;
                        }

                        // look for the tokens ' ', '>', '/' or '?'
                        // todo: check for '?' because of the xml version statement: <?xml version="1.0" encoding="UTF-8"?>
                        let chunk = _mm_loadu_si128(ptr_next as *const _); // 6
                        let mask = _mm_movemask_epi8(_mm_or_si128(
                            _mm_or_si128(
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'>' as i8)), // 0x3e
                            ),
                            _mm_or_si128(
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'/' as i8)), // 0x2f
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'?' as i8)), // 0x3f
                            ),
                        )); // 10
                        if mask != 0 {
                            offset += mask.trailing_zeros() as usize;
                            // make sure offset is inside the input bounds
                            if rem < offset {
                                offset = rem;
                            }
                            break;
                        } else {
                            // move next
                            offset += 16;
                        }
                    }

                    // consume the input
                    let name = str_from_raw_parts(ptr, offset);
                    ptr = ptr.add(offset);

                    // todo: handle when `name` is empty, should it be considered part of a text block?

                    if name.starts_with("!--") {
                        // this check if very importat to avoid unecessary iterations when looking for the end of comment ("-->")
                        if *ptr == b'>' {
                            // small comment
                            if name.ends_with("--") {
                                // consume '>'
                                ptr = ptr.add(1);
                                // invoke comment event
                                let text = str_from_raw_parts(name.as_ptr().add(3), name.len() - 5);
                                (f)(XmlEvent::Comment { text });
                                // keep going
                                continue;
                            } else {
                                // consume '>' by just applying a offset
                                offset += 1;
                            }
                        }

                        // undo the offset, but don't include the "!--" at the begin of the comment
                        offset -= 3;
                        ptr = ptr.sub(offset);

                        // it might be needed for many iterations
                        let rem = ptr_end.offset_from(ptr) as usize;

                        // look for the "-->"
                        loop {
                            // out of bounds check
                            let ptr_next = ptr.add(offset);
                            if ptr_next >= ptr_end {
                                // use the remainig text to rise the event
                                offset = rem;
                                break;
                            }

                            // look for the token '>'
                            let chunk = _mm_loadu_si128(ptr_next as *const _); // 6
                            let mask =
                                _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'>' as i8))); // 4

                            if mask != 0 {
                                offset += mask.trailing_zeros() as usize;
                                if rem < offset {
                                    // consider the remaning of the input as a comment and end the parsing right here
                                    let text = str_from_raw_parts(ptr, rem);
                                    (f)(XmlEvent::Comment { text });
                                    (f)(XmlEvent::EOF);
                                    return;
                                }

                                if str_from_raw_parts(ptr, offset).ends_with("--") {
                                    let text = str_from_raw_parts(ptr, offset - 2);
                                    (f)(XmlEvent::Comment { text });
                                    // move next make sure to consume the '>' as well
                                    ptr = ptr.add(offset + 1);
                                    break;
                                }

                                // keep going because '>' isn't part of the end of comment token "-->"
                            } else {
                                // move next
                                offset += 16;
                            }
                        }
                    } else {
                        (f)(XmlEvent::PushElement { name });

                        let token = *ptr;
                        if token == b'/' || token == b'?' {
                            // todo: check if should emit and pop event in the case `<r/>`
                        }

                        // todo: emit attribute events
                        // name *[' '] '=' *[' '] '\"' [text] '\"' (' ' | '\n' | '\t' | '>' | '?' | '/')
                    }
                } else {
                    let mut offset = 0;
                    let rem = ptr_end.offset_from(ptr) as usize;

                    // text until it finds a '\n' or a '<'
                    loop {
                        // out of bounds check
                        let ptr_next = ptr.add(offset);
                        if ptr_next >= ptr_end {
                            // use the remainig text to rise the event
                            offset = rem;
                            break;
                        }

                        // look for the tokens '\n' or '<'
                        let chunk = _mm_loadu_si128(ptr_next as *const _); // 6
                        let mask = _mm_movemask_epi8(_mm_or_si128(
                            _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\n' as i8)), // 0x0A
                            _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'<' as i8)),  // 0x3c
                        )); // 6
                        if mask != 0 {
                            offset += mask.trailing_zeros() as usize;
                            // make sure offset is inside the input bounds
                            if rem < offset {
                                offset = rem;
                            }
                            break;
                        } else {
                            // move next
                            offset += 16;
                        }
                    }

                    // todo: find a better altenative for `trim_end`
                    let text = str_from_raw_parts(ptr, offset).trim_end();
                    (f)(XmlEvent::Text { text });

                    // move next
                    ptr = ptr.add(offset);
                }
            } else {
                // move next
                ptr = ptr.add(16);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_xml, XmlEvent};

    fn assert_xml<'a>(xml: &str, events: impl AsRef<[XmlEvent<'a>]>) {
        let mut rem = events.as_ref();
        parse_xml(xml, |event| {
            let (head, tail) = rem.split_at(1);
            assert_eq!(event, head[0]);
            rem = tail;
        });
    }

    #[test]
    fn bounds_check() {
        let (xml, _) = "   <".split_at(3);
        assert_eq!(xml, "   ");
        assert_xml(xml, [XmlEvent::EOF]);

        assert_xml(
            "   <r",
            [XmlEvent::PushElement { name: "r" }, XmlEvent::EOF],
        );
    }

    #[test]
    fn comments() {
        assert_xml(
            "   <!--bla-->",
            [XmlEvent::Comment { text: "bla" }, XmlEvent::EOF],
        );
        assert_xml(
            "<!--\n\ta much larger comment\n\tthat spawns across more than one line\n-->",
            [
                XmlEvent::Comment {
                    text: "\n\ta much larger comment\n\tthat spawns across more than one line\n",
                },
                XmlEvent::EOF,
            ],
        );
    }

    #[test]
    fn text() {
        // assert_xml(
        //     "<a>  some text  </a>",
        //     [
        //         XmlEvent::PushElement { name: "a" },
        //         XmlEvent::Text { text: "some text" },
        //         XmlEvent::PopElement { name: Some("a") },
        //         XmlEvent::EOF,
        //     ],
        // );

        // assert_xml(
        //     "<a>some text</a>",
        //     [
        //         XmlEvent::PushElement { name: "a" },
        //         XmlEvent::Text { text: "some text" },
        //         XmlEvent::PopElement { name: Some("a") },
        //         XmlEvent::EOF,
        //     ],
        // );

        assert_xml(
            " some  <!--   not  -->   text   ",
            [
                XmlEvent::Text { text: "some" },
                XmlEvent::Comment { text: "   not  " },
                XmlEvent::Text { text: "text" },
                XmlEvent::EOF,
            ],
        );

        // assert_xml(
        //     "<a>  some <!-- not --> text  </a>",
        //     [
        //         XmlEvent::PushElement { name: "a" },
        //         XmlEvent::Text { text: "some" },
        //         XmlEvent::Text { text: "text" },
        //         XmlEvent::PopElement { name: Some("a") },
        //         XmlEvent::EOF,
        //     ],
        // );

        assert_xml(
            "  text\n only  ",
            [
                XmlEvent::Text { text: "text" },
                XmlEvent::Text { text: "only" },
                XmlEvent::EOF,
            ],
        );

        // // doesnt support embedding '<' or '>' during the texts
        // assert_xml(
        //     "<a>20 &lt; 30</a>",
        //     [
        //         XmlEvent::PushElement { name: "a" },
        //         XmlEvent::Text { text: "20 &lt; 30" },
        //         XmlEvent::PopElement { name: Some("a") },
        //     ],
        // );
    }

    // #[test]
    // fn elements() {
    //     super::parse_xml("<r></r>", |event| {
    //         panic!("{:?}", event);
    //     });
    // }
}
