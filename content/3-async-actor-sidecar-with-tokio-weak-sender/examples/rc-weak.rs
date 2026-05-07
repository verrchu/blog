use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    // The parent keeps its children alive: strong edge, downward.
    children: RefCell<Vec<Rc<Node>>>,
    // The child points back at its parent without keeping it alive: weak
    // edge, upward. This is what stops the two from forming a leak.
    parent: RefCell<Weak<Node>>,
}

pub fn demo() {
    let parent = Rc::new(Node {
        children: RefCell::new(Vec::new()),
        parent: RefCell::new(Weak::new()),
    });

    let child = Rc::new(Node {
        children: RefCell::new(Vec::new()),
        parent: RefCell::new(Rc::downgrade(&parent)),
    });

    parent.children.borrow_mut().push(Rc::clone(&child));

    // Only strong handles govern deallocation. The child's back-pointer is
    // weak, so the parent's strong count is still 1, not 2.
    assert_eq!(Rc::strong_count(&parent), 1);
    assert_eq!(Rc::weak_count(&parent), 1);

    // upgrade() works while the parent is alive...
    assert!(child.parent.borrow().upgrade().is_some());

    drop(parent);

    // ...and returns None once the last strong handle is gone, regardless of
    // how many weak handles still point at it.
    assert!(child.parent.borrow().upgrade().is_none());
}
