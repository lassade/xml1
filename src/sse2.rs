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

// main goal is to find the folling tokens '<' '=' ' ' '/' '?' '>'

const unsafe fn str_from_raw_parts<'a>(ptr: *const u8, len: usize) -> &'a str {
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
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

            // ignore whitespaces
            let space_mask = !_mm_movemask_epi8(_mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8))); // 4
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

                    // begin element
                    loop {
                        // out of bounds check
                        let ptr_next = ptr.add(offset);
                        if ptr_next >= ptr_end {
                            // use the remainig text to rise the event
                            offset = ptr_end.offset_from(ptr) as usize;
                            break;
                        }

                        // look for the tokens ' ', '>' or '/'
                        // todo: check for '?' because of the xml version statement: <?xml version="1.0" encoding="UTF-8"?>
                        let chunk = _mm_loadu_si128(ptr_next as *const _); // 6
                        let mask = _mm_movemask_epi8(_mm_or_si128(
                            _mm_or_si128(
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b' ' as i8)), // 0x20
                                _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'>' as i8)), // 0x3e
                            ),
                            _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'/' as i8)), // 0x2f
                        )); // 8

                        if mask != 0 {
                            offset += mask.trailing_zeros() as usize;
                            // limit the input
                            let len = ptr_end.offset_from(ptr) as usize;
                            if len < offset {
                                offset = len;
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
                        let len = ptr_end.offset_from(ptr) as usize;

                        // look for the "-->"
                        loop {
                            // out of bounds check
                            let ptr_next = ptr.add(offset);
                            if ptr_next >= ptr_end {
                                // use the remainig text to rise the event
                                offset = len;
                                break;
                            }

                            // look for the token '>'
                            let chunk = _mm_loadu_si128(ptr_next as *const _); // 6
                            let mask =
                                _mm_movemask_epi8(_mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'>' as i8))); // 4

                            if mask != 0 {
                                offset += mask.trailing_zeros() as usize;
                                if len < offset {
                                    // consider the remaning of the input as a comment and end the parsing right here
                                    let text = str_from_raw_parts(ptr, len);
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
                        // todo: emit attribute events
                        // todo: check if should emmit and pop event in the case `<r/>`
                    }
                } else {
                    // text until it finds a b'\n' or a b'<'
                    let mut element_offset = space_offset + 1;
                    loop {}
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
    fn comments_check() {
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

    // #[test]
    // fn elements() {
    //     super::parse_xml("<r></r>", |event| {
    //         panic!("{:?}", event);
    //     });
    // }
}
