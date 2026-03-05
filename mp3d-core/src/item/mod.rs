//! Items for a voxel engine.

use crate::block::Block;

/// A struct used for declaring different types of items on the fly. Mineplace provides some
/// already defined items and an array of the already defined items.
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub ident: &'static str,
    pub assoc_block: Option<&'static Block>,
    pub max_stack: u16,
}

impl Item {
    pub const AIR: Item = Item {
        ident: "air",
        assoc_block: Some(&Block::AIR),
        max_stack: 64,
    };

    pub const GRASS_BLOCK: Item = Item {
        ident: "grass_block",
        assoc_block: Some(&Block::GRASS),
        max_stack: 64,
    };

    pub const DIRT: Item = Item {
        ident: "dirt",
        assoc_block: Some(&Block::DIRT),
        max_stack: 64,
    };

    pub const STONE: Item = Item {
        ident: "stone",
        assoc_block: Some(&Block::STONE),
        max_stack: 64,
    };

    pub const GLUNGUS_BLOCK: Item = Item {
        ident: "glungus_block",
        assoc_block: Some(&Block::GLUNGUS),
        max_stack: 64,
    };

    pub const STONE_SLAB: Item = Item {
        ident: "stone_slab",
        assoc_block: Some(&Block::STONE_SLAB),
        max_stack: 64,
    };

    pub const ALL_ITEMS: &[Item] = &[
        Item::AIR,
        Item::GRASS_BLOCK,
        Item::DIRT,
        Item::STONE,
        Item::GLUNGUS_BLOCK,
        Item::STONE_SLAB,
    ];
}

/// A struct representing a stack of items, containing a reference to the item and the count of how
/// many of that item are in the stack. The count is limited by the max stack size of the item. An
/// empty stack is represented by an item of AIR and a count of 0.
#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    pub item: &'static Item,
    pub count: u16,
}

impl ItemStack {
    pub fn new(item: &'static Item, count: u16) -> Self {
        Self {
            item,
            count: count.min(item.max_stack),
        }
    }

    /// Creates an empty item stack with the item set to AIR and count set to 0.
    pub fn empty() -> Self {
        Self {
            item: &Item::AIR,
            count: 0,
        }
    }

    /// Checks if the item stack is empty by checking if the count is 0. The item can be AIR or any
    /// other item, but the count must be 0 for the stack to be considered empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Adds a specified count of items to the stack, ensuring that the total count does not exceed
    /// the max stack size of the item. The method returns the remainder item stack that couldn't
    /// be added if the count exceeds the max stack size.
    pub fn add(&mut self, count: u16) -> ItemStack {
        let available_space = self.item.max_stack - self.count;
        let to_add = count.min(available_space);
        self.count += to_add;
        let rem = count - to_add;
        if rem > 0 {
            ItemStack {
                item: self.item,
                count: count - to_add,
            }
        } else {
            ItemStack::empty()
        }
    }

    /// Adds another item stack to this stack, ensuring that the items in both stacks are the same
    /// or that at least one of the stacks is empty. The method returns the remainder item stack
    /// that couldn't be added if the count exceeds the max stack size or if the items in the
    /// stacks are different and neither stack is empty. If the items in the stacks are different
    /// and at least one of the stacks is empty, then the method will add the items to the empty
    /// stack and return an empty stack as the remainder. If the items in the stacks are the same,
    /// then the method will add the counts together and return any remainder if the total count
    /// exceeds the max stack size.
    pub fn add_stack(&mut self, other: &ItemStack) -> ItemStack {
        if other.is_empty() {
            return ItemStack::empty();
        }

        if self.is_empty() {
            self.item = other.item;
        }

        if !std::ptr::eq(self.item, other.item) {
            return *other;
        }

        self.add(other.count)
    }

    /// Removes a specified count of items from the stack, ensuring that the total count does not
    /// go below 0. The method returns the item stack that was removed from the original stack. If
    /// the count to remove exceeds the current count in the stack, then all items in the stack are
    /// removed and the method returns an item stack with the same item and the count of items that
    /// were actually removed.
    pub fn remove(&mut self, count: u16) -> ItemStack {
        let to_remove = count.min(self.count);
        self.count -= to_remove;

        let removed_stack = if to_remove > 0 {
            ItemStack {
                item: self.item,
                count: to_remove,
            }
        } else {
            ItemStack::empty()
        };

        if self.count == 0 {
            self.item = &Item::AIR;
        }

        removed_stack
    }

    /// Takes some items from another stack to this stack, giving back the leftover or unmergeable
    /// items to the other stack.
    pub fn take_from(&mut self, other: &mut ItemStack, count: u16) {
        if !self.can_merge(other) {
            return; // Can't take from different items, return empty stack
        }

        let removed = other.remove(count);
        let remainder = self.add_stack(&removed);

        // Give leftover back to other
        if !remainder.is_empty() {
            other.add_stack(&remainder);
        }
    }

    /// Checks if this item stack can be merged with another item stack, which is true if the items
    /// in both stacks are the same or if at least one of the stacks is empty. Note that this
    /// method does not check if the total count of the merged stacks would exceed the max stack
    /// size of the item, it only checks if the items are compatible for merging.
    pub fn can_merge(&self, other: &ItemStack) -> bool {
        std::ptr::eq(self.item, other.item) || self.is_empty() || other.is_empty()
    }
}

/// A struct representing an inventory, storing 36 general purpose item stacks and one temporary
/// stack used for dragging items around in the UI.
#[derive(Clone, Debug)]
pub struct Inventory {
    pub main: [ItemStack; 36],
    pub temp: ItemStack,
}

impl Inventory {
    /// Makes a new inventory with all item stacks initialized to empty and the temporary stack
    /// also initialized to empty.
    pub fn new() -> Self {
        Self {
            main: [ItemStack::empty(); 36],
            temp: ItemStack::empty(),
        }
    }

    /// Takes the general slot into the temporary slot and leaves the remainder back to the general
    /// slot.
    pub fn take_into_temp(&mut self, index: usize) {
        let count = self.main[index].count;
        self.temp.take_from(&mut self.main[index], count);
    }

    /// Takes the temporary slot into the general slot and leaves the remainder back to the
    /// temporary slot.
    pub fn take_from_temp(&mut self, index: usize) {
        let count = self.temp.count;
        self.main[index].take_from(&mut self.temp, count);
    }

    /// Simulates a click on a general slot.
    pub fn click(&mut self, index: usize, right: bool) {
        if right {
            // Right click: If the temporary stack is empty, halve the general slot stack and take
            // the halved amount into the temporary stack. If the temporary stack is not empty,
            // take one item from the temporary stack into the general slot.
            if self.temp.is_empty() {
                let half_count = (self.main[index].count + 1) / 2;
                self.temp.take_from(&mut self.main[index], half_count);
            } else {
                self.main[index].take_from(&mut self.temp, 1);
            }
        } else {
            // // Left click: Swap the temporary stack with the general slot stack
            // std::mem::swap(&mut self.main[index], &mut self.temp);
            if self.temp.is_empty() {
                self.take_into_temp(index);
            } else {
                self.take_from_temp(index);
            }
        }
    }
}


