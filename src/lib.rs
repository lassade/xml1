mod chars;
use chars::Chars;

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
    Chars {
        text: &'a str,
    },
}

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
        // expects `!--`
        self.input.next();
        self.input.next();
        self.input.next();

        loop {
            let rem = self.input.tail();
            if rem.starts_with("-->") {
                self.input.next();
                self.input.next();
                self.input.next();
                break;
            } else {
                if self.input.next() == None || rem.len() < "-->".len() {
                    break;
                }
            }
        }
    }

    fn document_events(&mut self) -> Option<XmlEvent<'a>> {
        loop {
            self.ignore_whitespace();

            println!("'{:?}', \"{}\"", self.input.head(), self.input.tail());

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
                Some(ch) => {
                    panic!("unexpected char `{}`", ch);
                }
            }
        }
    }

    fn push_element(&mut self) -> Option<XmlEvent<'a>> {
        let cursor = self.input.cursor();
        while let Some(ch) = self.input.head() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
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
                    todo!()
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
                Some(ch) => {
                    if ch.is_alphanumeric() || ch == '_' || ch == '_' {
                        return self.push_attr();
                    } else {
                        panic!("unexpected char `{}`", ch);
                    }
                }
            }
        }
    }

    fn push_attr(&mut self) -> Option<XmlEvent<'a>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn basic() {
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
    }

    #[test]
    fn comments() {
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
    }
}
