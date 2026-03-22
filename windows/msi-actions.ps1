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

$exe = Join-Path $PSScriptRoot 'winget-auto-upgrade.exe'

if ($Uninstall) {
    Unregister-ScheduledTask -TaskName 'winget-auto-upgrade'      -Confirm:$false -ErrorAction SilentlyContinue
    Unregister-ScheduledTask -TaskName 'winget-auto-upgrade-user' -Confirm:$false -ErrorAction SilentlyContinue
} else {
    $action   = New-ScheduledTaskAction -Execute $exe
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew

    # SYSTEM context — daily at 06:00 and on any user logon, two triggers one task
    $triggers  = @(
        (New-ScheduledTaskTrigger -Daily -At '06:00'),
        (New-ScheduledTaskTrigger -AtLogOn)
    )
    $principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -RunLevel Highest
    Register-ScheduledTask -TaskName 'winget-auto-upgrade' `
        -Action $action -Trigger $triggers -Principal $principal -Settings $settings -Force | Out-Null

    # User context — trigger-less, started explicitly by the SYSTEM task at end of its run.
    # Runs as Authenticated Users (S-1-5-11) with highest available privileges
    # so that Administrator-installed packages can also be upgraded.
    $principal = New-ScheduledTaskPrincipal -GroupId 'S-1-5-11' -RunLevel Highest
    Register-ScheduledTask -TaskName 'winget-auto-upgrade-user' `
        -Action $action -Principal $principal -Settings $settings -Force | Out-Null
}
