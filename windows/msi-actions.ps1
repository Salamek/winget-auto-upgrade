#Requires -Version 5.0
<#
.SYNOPSIS
    MSI custom action script for winget-auto-upgrade.
    Called by the installer on install and uninstall.

.PARAMETER Uninstall
    When set, removes the scheduled tasks instead of creating them.
#>
param(
    [switch]$Uninstall
)

$installPath = $PSScriptRoot
$exe         = Join-Path $installPath 'winget-auto-upgrade.exe'
$taskPath    = '\winget-auto-upgrade\'

if ($Uninstall) {
    Unregister-ScheduledTask -TaskName 'System' -TaskPath $taskPath -Confirm:$false -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName 'User'   -TaskPath $taskPath -Confirm:$false -ErrorAction SilentlyContinue
} else {
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew

    # SYSTEM context — daily at 06:00 and on any user logon, two triggers one task
    $action    = New-ScheduledTaskAction -Execute $exe -WorkingDirectory $installPath
    $triggers  = @(
        (New-ScheduledTaskTrigger -Daily -At '06:00'),
        (New-ScheduledTaskTrigger -AtLogOn)
    )
    $principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest
    Register-ScheduledTask -TaskName 'System' -TaskPath $taskPath `
        -Action $action -Trigger $triggers -Principal $principal -Settings $settings -Force | Out-Null

    # User context — trigger-less, started explicitly by the SYSTEM task at end of its run.
    # Launched via conhost --headless to suppress the console window in the user's session.
    # Runs as Authenticated Users (S-1-5-11) with highest available privileges
    # so that Administrator-installed packages can also be upgraded.
    $action    = New-ScheduledTaskAction -Execute 'conhost.exe' `
                     -Argument "--headless `"$exe`"" `
                     -WorkingDirectory $installPath
    $principal = New-ScheduledTaskPrincipal -GroupId 'S-1-5-11' -RunLevel Highest
    Register-ScheduledTask -TaskName 'User' -TaskPath $taskPath `
        -Action $action -Principal $principal -Settings $settings -Force | Out-Null
}
