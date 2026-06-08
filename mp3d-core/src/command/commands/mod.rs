use crate::command::CommandManager;

mod give;
mod say;
mod seed;
mod setblock;
mod tp;
mod tps;

pub fn init_command_mgr(mgr: &mut CommandManager) {
    mgr.register(give::GiveCommand);
    mgr.register(say::SayCommand);
    mgr.register(seed::SeedCommand);
    mgr.register(setblock::SetBlockCommand);
    mgr.register(tp::TpCommand);
    mgr.register(tps::TpsCommand);
}
