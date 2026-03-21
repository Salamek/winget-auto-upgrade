use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct HookContext<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub source: &'a str,
    pub scope: &'a str,
    pub version: &'a str,
    pub available_version: &'a str,
}

fn interpolate(template: &str, ctx: &HookContext) -> String {
    template
        .replace("{id}", ctx.id)
        .replace("{name}", ctx.name)
        .replace("{source}", ctx.source)
        .replace("{scope}", ctx.scope)
        .replace("{version}", ctx.version)
        .replace("{available_version}", ctx.available_version)
}

pub fn run(hook_path: &Path, args_template: &str, ctx: &HookContext) -> Result<()> {
    let args_str = interpolate(args_template, ctx);
    let args: Vec<&str> = args_str.split_whitespace().collect();

    let status = Command::new(hook_path)
        .args(&args)
        .status()
        .with_context(|| format!("Failed to run hook: {}", hook_path.display()))?;

    if !status.success() {
        anyhow::bail!(
            "Hook {} exited with status {}",
            hook_path.display(),
            status
        );
    }
    Ok(())
}
