# Generates the app icon and a legibility preview at real tray sizes.
#
# The binding constraint is the 16px system-tray icon, not the 1024px source:
# anything that merges into a blob at 16px is a failed icon no matter how good
# it looks large. Hence the generous gap between the selection brackets and the
# text bars, and the deliberately heavy stroke.
#
# Concept: selection brackets (the capture gesture) framing two text bars —
# white for the source line, accent for the translated one. No letters, because
# the target language is a setting and the app is not tied to any one script.
#
#   powershell -ExecutionPolicy Bypass -File scripts/make-icon.ps1

Add-Type -AssemblyName System.Drawing

$Accent = [System.Drawing.Color]::FromArgb(255, 56, 189, 248)   # sky-400
$Plate  = [System.Drawing.Color]::FromArgb(255, 24, 24, 27)     # zinc-900
$Text   = [System.Drawing.Color]::FromArgb(255, 250, 250, 250)

function New-RoundedPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $r * 2
  $p.AddArc($x, $y, $d, $d, 180, 90)
  $p.AddArc($x + $w - $d, $y, $d, $d, 270, 90)
  $p.AddArc($x + $w - $d, $y + $h - $d, $d, $d, 0, 90)
  $p.AddArc($x, $y + $h - $d, $d, $d, 90, 90)
  $p.CloseFigure()
  return $p
}

function New-AppIcon([int]$S) {
  $bmp = New-Object System.Drawing.Bitmap($S, $S)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::Transparent)
  $u = $S / 1024.0

  # Plate. Small inset so the mark uses as much of the 16px cell as possible.
  $inset = 28 * $u
  $side = $S - 2 * $inset
  $g.FillPath((New-Object System.Drawing.SolidBrush($Plate)),
              (New-RoundedPath $inset $inset $side $side (205 * $u)))

  # Text bars. Sized to carry the mark rather than decorate it: too small and
  # they collapse into an "equals sign" dot at 16px, which says nothing.
  $barH = 122 * $u
  $barX = 336 * $u
  foreach ($bar in @(@{ y = 408; w = 352; c = $Text }, @{ y = 566; w = 244; c = $Accent })) {
    $g.FillPath((New-Object System.Drawing.SolidBrush($bar.c)),
                (New-RoundedPath $barX ($bar.y * $u) ($bar.w * $u) $barH ($barH / 2)))
  }

  # Selection brackets.
  $pen = New-Object System.Drawing.Pen($Accent, [float](56 * $u))
  $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
  $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round

  $L = 158 * $u; $R = $S - $L
  $T = 220 * $u; $B = $S - $T
  $a = 150 * $u

  function Corner([float]$cx, [float]$cy, [float]$dx, [float]$dy) {
    $g.DrawLines($pen, @(
      (New-Object System.Drawing.PointF($cx, ($cy + $dy * $a))),
      (New-Object System.Drawing.PointF($cx, $cy)),
      (New-Object System.Drawing.PointF(($cx + $dx * $a), $cy))
    ))
  }
  Corner $L $T 1 1
  Corner $R $T -1 1
  Corner $L $B 1 -1
  Corner $R $B -1 -1

  $g.Dispose()
  return $bmp
}

$root = Split-Path -Parent $PSScriptRoot
$icon = New-AppIcon 1024
Remove-Item "$root\app-icon.png" -Force -ErrorAction SilentlyContinue
$icon.Save("$root\app-icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
$icon.Dispose()

# Preview at the sizes Windows actually renders, so 16px legibility is judged
# rather than assumed.
$sizes = @(16, 20, 24, 32, 48, 64, 128, 256)
$pad = 18
$tw = [int](($sizes | Measure-Object -Sum).Sum + $pad * ($sizes.Count + 1))
$prev = New-Object System.Drawing.Bitmap([int]$tw, [int]300)
$pg = [System.Drawing.Graphics]::FromImage($prev)
$pg.Clear([System.Drawing.Color]::FromArgb(255, 250, 250, 250))
$pg.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$pg.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

$src = [System.Drawing.Image]::FromFile("$root\app-icon.png")
$font = New-Object System.Drawing.Font("Segoe UI", 9)
$x = $pad
foreach ($s in $sizes) {
  $pg.DrawImage($src, (New-Object System.Drawing.Rectangle([int]$x, [int](265 - $s), [int]$s, [int]$s)))
  $pg.DrawString("${s}px", $font, [System.Drawing.Brushes]::DimGray, [float]$x, [float]272)
  $x = [int]($x + $s + $pad)
}
$pg.Dispose(); $src.Dispose()
$prev.Save("$root\icon-preview.png", [System.Drawing.Imaging.ImageFormat]::Png)
$prev.Dispose()

Write-Output "app-icon.png and icon-preview.png regenerated"
