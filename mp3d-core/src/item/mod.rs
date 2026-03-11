//! Items for a voxel engine.

use crate::block::Block;

mod save_impls;

/// A struct used for declaring different types of items on the fly. Mineplace provides some
/// already defined items and an array of the already defined items.
#[derive(Clone, Copy, Debug)]
pub struct Item {
    pub ident: &'static str,
    pub assoc_block: Option<&'static Block>,
    pub max_stack: u16,
}

impl Default for Item {
    fn default() -> Self {
        Item::AIR
    }
}

macro_rules! items {
    (
        $(
            $const_ident:ident => {
                $ident:literal $(x $stack:expr)?
                $(, block: $block:expr)?
            }
        ),* $(,)?
    ) => {
        impl Item {
            $(
                items!(@item $const_ident, $ident, $($stack)?, $($block)?);
            )*

            pub const ALL_ITEMS: &[Item] = &[
                $(Item::$const_ident),*
            ];

            pub fn from_ident(ident: &str) -> Option<&'static Item> {
                match ident {
                    $(
                        $ident => Some(&Item::$const_ident),
                    )*
                    _ => None,
                }
            }
        }
    };

    (@item $const_ident:ident, $ident:literal, $stack:expr, $block:expr) => {
        pub const $const_ident: Item = Item {
            ident: $ident,
            assoc_block: $block,
            max_stack: $stack,
        };
    };

    (@item $const_ident:ident, $ident:literal, $stack:expr,) => {
        pub const $const_ident: Item = Item {
            ident: $ident,
            assoc_block: None,
            max_stack: $stack,
        };
    };

    (@item $const_ident:ident, $ident:literal,, $block:expr) => {
        pub const $const_ident: Item = Item {
            ident: $ident,
            assoc_block: $block,
            max_stack: 64,
        };
    };

    (@item $const_ident:ident, $ident:literal,,) => {
        pub const $const_ident: Item = Item {
            ident: $ident,
            assoc_block: None,
            max_stack: 64,
        };
    };
}

items!(
    AIR => { "air", block: Some(&Block::AIR) },
    GRASS_BLOCK => { "grass_block", block: Some(&Block::GRASS) },
    DIRT => { "dirt", block: Some(&Block::DIRT) },
    STONE => { "stone", block: Some(&Block::STONE) },
    COBBLESTONE => { "cobblestone", block: Some(&Block::COBBLESTONE) },
    LOG => { "log", block: Some(&Block::LOG) },
    LEAVES => { "leaves", block: Some(&Block::LEAVES) },
    GLUNGUS_BLOCK => { "glungus_block", block: Some(&Block::GLUNGUS) },
    STONE_SLAB => { "stone_slab", block: Some(&Block::STONE_SLAB) },
);

/// A struct representing a stack of items, containing a the item and the count of how many of
/// that item are in the stack. The count is limited by the max stack size of the item. An empty
/// stack is represented by an item of AIR and a count of 0.
#[derive(Clone, Copy, Debug, Default)]
pub struct ItemStack {
    pub item: Item,
    pub count: u16,
}

impl ItemStack {
    pub fn new(item: Item, count: u16) -> Self {
        Self {
            item,
            count: count.min(item.max_stack),
        }
    }

    /// Creates an empty item stack with the item set to AIR and count set to 0.
    pub fn empty() -> Self {
        Self {
            item: Item::AIR,
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

        if self.item.ident != other.item.ident {
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
            self.item = Item::AIR;
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
        self.item.ident == other.item.ident || self.is_empty() || other.is_empty()
    }
}

/// A struct representing an inventory, storing 36 general purpose item stacks and one temporary
/// stack used for dragging items around in the UI.
#[derive(Clone, Debug)]
pub struct Inventory {
    pub main: [ItemStack; 36],
    pub temp: ItemStack,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
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
                let half_count = self.main[index].count.div_ceil(2);
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

    /// Returns all slots, including the temporary slot, as a single vector of item stacks. The
    /// temporary slot is included at the end of the vector after all the general slots.
    pub fn slots(&self) -> Vec<ItemStack> {
        let mut slots = self.main.to_vec();
        slots.push(self.temp);
        slots
    }

    /// Returns mutable references to all slots, including the temporary slot, as a single vector
    /// of mutable item stacks. The temporary slot is included at the end of the vector after all
    /// the general slots.
    pub fn slots_mut(&mut self) -> Vec<&mut ItemStack> {
        let mut slots: Vec<&mut ItemStack> = self.main.iter_mut().collect();
        slots.push(&mut self.temp);
        slots
    }

    /// Searches for a place to put the given item stack in the inventory and adds it to the first
    /// suitable slot.
    pub fn add_stack_single(&mut self, stack: ItemStack) {
        for slot in self.slots_mut() {
            if slot.can_merge(&stack) {
                let remainder = slot.add_stack(&stack);
                if remainder.is_empty() {
                    break;
                }
            }
        }
    }

    /// Adds a specified count of items of a given item to the inventory, splitting it into
    /// multiple stacks if necessary.
    pub fn add_stack(&mut self, item: Item, mut count: u16) {
        let n_stacks = count.div_ceil(item.max_stack);
        for _ in 0..n_stacks {
            let stack_count = count.min(item.max_stack);
            self.add_stack_single(ItemStack::new(item, stack_count));
            count -= stack_count;
        }
    }

    /// Gets a specified hotbar slot. The hotbar consists of the last 9 slots of the general
    /// inventory, so the index is adjusted accordingly.
    pub fn hotbar_slot(&self, index: usize) -> &ItemStack {
        &self.main[3 * 9 + index]
    }
}
