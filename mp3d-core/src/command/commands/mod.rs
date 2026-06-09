use crate::command::CommandManager;

mod clear;
mod give;
mod help;
mod say;
mod seed;
mod setblock;
mod tp;
mod tps;

pub fn init_command_mgr(mgr: &mut CommandManager) {
    mgr.register(clear::ClearCommand);
    mgr.register(give::GiveCommand);
    mgr.register(help::HelpCommand);
    mgr.register(say::SayCommand);
    mgr.register(seed::SeedCommand);
    mgr.register(setblock::SetBlockCommand);
    mgr.register(tp::TpCommand);
    mgr.register(tps::TpsCommand);
}
