use crate::cursor::Cursor;
use crate::pager::Pager;
use crate::Row;

pub const NODE_SIZE: usize = 4096;
pub const NODE_TYPE_OFFSET: usize = 0;
pub const IS_ROOT_OFFSET: usize = 1;
pub const PARENT_OFFSET: usize = 2;
pub const NUM_CELLS_OFFSET: usize = 6;
pub const CELL_KEY_SIZE: usize = 4;
pub const CELL_VALUE_SIZE: usize = 291;
pub const CELL_OFFSET: usize = 10;
pub const CELL_SIZE: usize = CELL_VALUE_SIZE + CELL_KEY_SIZE;

pub struct KeyValuePair<K, V> {
    pub key: K,
    pub value: V,
}

pub enum NodeType<K, V> {
    Internal,
    Leaf(Vec<KeyValuePair<K, V>>),
}

pub struct BTree {
    root: Node<usize, Row>,
    pager: Pager,
}

impl BTree {
    pub fn new(mut pager: Pager) -> Self {
        let root = if pager.num_pages() == 0 {
            let mut root_node = pager.get_page(0);
            root_node.is_root = true;
            pager.commit_page(&root_node);
            root_node
        } else {
            pager.get_page(0)
        };

        Self { root, pager }
    }

    pub fn get(&self, page_num: usize, cell_num: usize) -> Row {
        let node = self.pager.get_page(page_num);
        node.get(cell_num).as_ref().unwrap().value.clone()
    }

    pub fn insert(&mut self, cursor: &Cursor, value: Row) -> bool {
        let mut node = self.pager.get_page(cursor.page_num());
        if node.num_cells >= 12 {
            let mut new_node = node.split(self.pager.num_pages());
            let mut new_parent = Node::internal();

            return false;
        } else {
            if node.insert(cursor.cell_num(), value.id as usize, value) {
                self.pager.commit_page(&node);
                if node.is_root {
                    self.root = node;
                }
                return true;
            }
            return false;
        }
    }

    pub fn root(&self) -> &Node<usize, Row> {
        &self.root
    }

    pub fn close(&mut self) {
        self.pager.close()
    }

    pub fn find(&self, key: usize) -> Result<Cursor, Cursor> {
        self.root
            .find(key)
            .map(|(page_num, cell_num)| {
                Cursor::new(
                    page_num,
                    cell_num,
                    self.pager.num_pages() == page_num && cell_num >= 12,
                )
            })
            .map_err(|(page_num, insert_cell_num)| {
                Cursor::new(
                    page_num,
                    insert_cell_num,
                    self.pager.num_pages() == page_num && insert_cell_num >= 12,
                )
            })
    }
}

pub struct Node<K: Ord, V> {
    pub(crate) is_root: bool,
    pub(crate) node_type: NodeType<K, V>,
    pub(crate) parent_offset: Option<usize>,
    pub(crate) num_cells: usize,
    pub(crate) page_num: usize,
}

impl<K: Ord, V> Node<K, V> {
    pub fn leaf() -> Self {
        Self {
            is_root: false,
            node_type: NodeType::Leaf(Vec::new()),
            parent_offset: None,
            num_cells: 0,
            page_num: 0,
        }
    }

    pub fn leaf_with_children(children: impl Iterator<Item = KeyValuePair<K, V>>) -> Self {
        let children = Vec::from_iter(children);
        let num_cells = children.len();
        Self {
            is_root: false,
            node_type: NodeType::Leaf(children),
            parent_offset: None,
            num_cells,
            page_num: 0,
        }
    }

    pub fn internal() -> Self {
        Self {
            is_root: false,
            node_type: NodeType::Internal,
            parent_offset: None,
            num_cells: 0,
            page_num: 0,
        }
    }

    pub fn get(&self, cell_num: usize) -> Option<&KeyValuePair<K, V>> {
        if let NodeType::Leaf(ref cells) = self.node_type {
            return cells.get(cell_num);
        }
        None
    }

    pub fn insert(&mut self, cell_num: usize, key: K, value: V) -> bool {
        match self.node_type {
            NodeType::Leaf(ref mut cells) => {
                if cells.len() >= 12 {
                    return false;
                }
                cells.insert(cell_num, KeyValuePair { key, value });
                self.num_cells += 1;
                return true;
            }
            _ => todo!(),
        }

        false
    }

    pub fn find(&self, key: K) -> Result<(usize, usize), (usize, usize)> {
        match self.node_type {
            NodeType::Leaf(ref cells) => {
                if self.num_cells >= 12 {
                    return Err((usize::MAX, 0));
                }
                cells
                    .binary_search_by(|kv| kv.key.cmp(&key))
                    .map(|ok| (self.page_num, ok))
                    .map_err(|err| (self.page_num, err))
            }
            _ => todo!(),
        }
    }

    pub fn split(&mut self, new_page_num: usize) -> Node<K, V> {
        if let NodeType::Leaf(ref mut cells) = self.node_type {
            let upper = cells.split_off(cells.len() / 2);
            let mut new_node = Node::leaf_with_children(upper.into_iter());
            new_node.page_num = new_page_num;
            return new_node;
        }
        todo!()
    }
}
