use std::cell::RefCell;
use std::collections::BTreeMap;

#[derive(Default)]
struct Node {
    index: usize,
    fields: BTreeMap<u32, Node>,
}

/// State shared by field occurrences during one message decode.
/// Field paths borrow stack frames; only fixed-array cursors allocate storage.
#[doc(hidden)]
pub struct DecodeState<'a>(State<'a>);

enum State<'a> {
    Root(RefCell<Option<Box<Node>>>),
    Field(&'a DecodeState<'a>, u32),
}

impl Default for DecodeState<'_> {
    fn default() -> Self {
        Self(State::Root(RefCell::new(None)))
    }
}

impl DecodeState<'_> {
    pub const fn field(&self, tag: u32) -> DecodeState<'_> {
        DecodeState(State::Field(self, tag))
    }

    fn root(&self) -> &RefCell<Option<Box<Node>>> {
        match &self.0 {
            State::Root(root) => root,
            State::Field(parent, _) => parent.root(),
        }
    }

    fn node<'n>(&self, root: &'n mut Node) -> &'n mut Node {
        match &self.0 {
            State::Root(_) => root,
            State::Field(parent, tag) => parent.node(root).fields.entry(*tag).or_default(),
        }
    }

    fn existing_node<'n>(&self, root: &'n mut Node) -> Option<&'n mut Node> {
        match &self.0 {
            State::Root(_) => Some(root),
            State::Field(parent, tag) => parent.existing_node(root)?.fields.get_mut(tag),
        }
    }

    pub fn clear(&self) {
        let mut storage = self.root().borrow_mut();
        if let Some(root) = storage.as_deref_mut()
            && let Some(node) = self.existing_node(root)
        {
            *node = Node::default();
        }
    }

    pub(crate) fn index(&self) -> usize {
        self.root().borrow_mut().as_deref_mut().and_then(|root| self.existing_node(root)).map_or(0, |node| node.index)
    }

    pub(crate) fn set_index(&self, index: usize) {
        self.node(self.root().borrow_mut().get_or_insert_with(Box::default)).index = index;
    }
}
