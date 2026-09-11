Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Force -Path 'src-tauri/icons'

$bmp = New-Object System.Drawing.Bitmap 128, 128
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$bgBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(15, 23, 42))
$g.FillRectangle($bgBrush, 0, 0, 128, 128)

$accentBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(99, 102, 241))
$g.FillEllipse($accentBrush, 14, 14, 100, 100)

$font = New-Object System.Drawing.Font('Segoe UI', 52, [System.Drawing.FontStyle]::Bold)
$textBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$format = New-Object System.Drawing.StringFormat
$format.Alignment = [System.Drawing.StringAlignment]::Center
$format.LineAlignment = [System.Drawing.StringAlignment]::Center
$rect = New-Object System.Drawing.RectangleF(0, 0, 128, 128)
$g.DrawString('M', $font, $textBrush, $rect, $format)

$bmp.Save('src-tauri/icons/icon.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Save('src-tauri/icons/128x128.png', [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Save('src-tauri/icons/128x128@2x.png', [System.Drawing.Imaging.ImageFormat]::Png)

$bmp32 = New-Object System.Drawing.Bitmap($bmp, 32, 32)
$bmp32.Save('src-tauri/icons/32x32.png', [System.Drawing.Imaging.ImageFormat]::Png)

$hIcon = $bmp32.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($hIcon)
$fs = [System.IO.File]::OpenWrite('src-tauri/icons/icon.ico')
$icon.Save($fs)
$fs.Close()

$icon.Dispose()
$bmp32.Dispose()
$bmp.Dispose()
$g.Dispose()
Write-Host "Icons generated successfully."
