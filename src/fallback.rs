#![allow(unused)]

//use alloc::vec::Vec;
use core::ptr::null;

use crate::{str_from_range, XmlEvent};

struct Parser {
    ptr: *const u8,
    ptr_end: *const u8,
    line_count: usize,
    line_ptr: *const u8,
}

impl Parser {
    fn new() -> Self {
        Self {
            ptr: null(),
            ptr_end: null(),
            line_count: 0,
            line_ptr: null(),
        }
    }

    #[inline(always)]
    unsafe fn find_match(&mut self, f: impl Fn(u8) -> bool) -> bool {
        while self.ptr < self.ptr_end {
            if (f)(*self.ptr) {
                return true;
            } else {
                self.ptr = self.ptr.add(1);
            }
        }

        false
    }

    // todo: or enter
    unsafe fn ignore_space(&mut self) -> bool {
        self.find_match(|ch| !(ch == b' ' || ch == b'\t'))
    }

    unsafe fn find_space(&mut self) -> bool {
        self.find_match(|ch| ch == b' ' || ch == b'\t')
    }

    unsafe fn find_space_or_enter(&mut self) -> bool {
        self.find_match(|ch| ch == b' ' || ch == b'\t' || ch == b'\n')
    }

    #[inline(always)]
    fn enter(&mut self) {
        self.line_count += 1;
        self.line_ptr = self.ptr;
    }

    unsafe fn parse<'a>(&mut self, input: &'a str, mut f: impl FnMut(XmlEvent<'a>)) {
        self.ptr = input.as_bytes().as_ptr();
        self.ptr_end = self.ptr.add(input.len());
        self.line_ptr = self.ptr;

        while self.ptr < self.ptr_end {
            if !self.ignore_space() {
                // nothing left
                return;
            }

            let ch = *self.ptr;

            if ch == b'\n' {
                // consume '\n'
                self.ptr = self.ptr.add(1);

                self.enter();

                continue;
            }

            if ch == b'<' {
                // begin element

                // skip '<'
                self.ptr = self.ptr.add(1);

                // read tag
                let tag_ptr = self.ptr;
                if !self.find_space_or_enter() {
                    // empty tag, nothing left todo
                    return;
                }

                let name = str_from_range(tag_ptr, self.ptr);
                (f)(XmlEvent::PushElement { name });

                // handle new line
                if *self.ptr == b'\n' {
                    // consume '\n'
                    self.ptr = self.ptr.add(1);

                    self.enter();
                }

                // todo: read attributes
                while self.ptr < self.ptr_end {
                    if !self.ignore_space() {
                        // nothing left
                        return;
                    }

                    let ch = *self.ptr;

                    if ch == b'/' {
                        // skip '/'
                        self.ptr = self.ptr.add(1);
                        if self.ptr < self.ptr_end {
                            if *self.ptr == b'>' {
                                (f)(XmlEvent::PopElement { name: None });
                                // skip '>'
                                self.ptr = self.ptr.add(1);
                                continue;
                            } else {
                                panic!("expecting `/>`")
                            }
                        }
                    }

                    if ch == b'>' {
                        // done reading attributes

                        // skip '>'
                        self.ptr = self.ptr.add(1);
                        break;
                    }

                    if ch == b'=' || ch == b'\"' {
                        panic!("unexpected char");
                    }

                    // read attribute name
                    let attr_ptr = self.ptr;

                    let mut break_mask = 0;
                    while self.ptr < self.ptr_end {
                        let ch = *self.ptr;
                        if ch == b'=' || ch == b'=' {
                            break;
                        }
                        self.ptr = self.ptr.add(1);
                    }

                    if self.ptr >= self.ptr_end {
                        self.ptr = self.ptr_end;
                        let name = str_from_range(attr_ptr, self.ptr);
                        (f)(XmlEvent::Attr { name, value: None });
                        return;
                    }

                    if break_mask & 0b1_0000 != 0 {}
                }
            } else {
                // push text
            }
        }
    }
}
