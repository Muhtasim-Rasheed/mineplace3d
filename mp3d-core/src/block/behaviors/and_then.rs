use crate::block::OnClick;

pub fn on_click(first: impl OnClick, second: impl OnClick) -> Box<dyn OnClick> {
    Box::new(move |world, entity, pos, state, face| {
        if first(world, entity, pos, state, face) {
            return true;
        }
        second(world, entity, pos, state, face)
    })
}
