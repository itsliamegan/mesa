use std::fmt::{self, Display, Formatter};
use std::ops::{Index, Range};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SourceId(u32);

impl SourceId {
	pub fn new(i: u32) -> Self {
		Self(i)
	}

	pub fn index(self) -> usize {
		self.0 as usize
	}
}

#[derive(Debug)]
pub struct Source {
	id: SourceId,
	file: String,
	text: String,
}

impl Source {
	pub fn new(id: SourceId, file: String, text: String) -> Self {
		Self { id, file, text }
	}

	pub fn id(&self) -> SourceId {
		self.id
	}

	pub fn len(&self) -> usize {
		self.text.len()
	}

	pub fn loc(&self, pos: usize) -> Location {
		let mut i = 0;
		let mut lin = 1;
		let mut col = 1;
		while i < pos && i < self.text.len() {
			if self.text.as_bytes()[i] == b'\n' {
				lin += 1;
				col = 1;
			} else {
				col += 1;
			}
			i += 1;
		}
		Location {
			file: self.file.clone(),
			pos: Some((lin, col)),
		}
	}
}

impl Index<usize> for Source {
	type Output = u8;

	fn index(&self, idx: usize) -> &Self::Output {
		&self.text.as_bytes()[idx]
	}
}

impl Index<Range<usize>> for Source {
	type Output = str;

	fn index(&self, idx: Range<usize>) -> &Self::Output {
		&self.text[idx]
	}
}

#[derive(Debug)]
pub struct Location {
	file: String,
	pos: Option<(usize, usize)>,
}

impl Location {
	fn file(file: String) -> Self {
		Self { file, pos: None }
	}
}

impl Display for Location {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
		match self.pos {
			Some((lin, col)) => write!(f, "{}:{},{}", self.file, lin, col),
			None => write!(f, "{}", self.file),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
	pub src: SourceId,
	pub start: usize,
	pub end: usize,
}
