use glam::IVec3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StructureData {
    Tree { trunk_height: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Structure {
    pub data: StructureData,
    pub pos: IVec3,
}
