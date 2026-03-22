$CargoExe = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
& $CargoExe build --release
