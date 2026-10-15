param([string]$OutputDirectory = "$PSScriptRoot/../apps/desktop/public/brand")
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$axOut = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($axOut) | Out-Null
$axFont = [Drawing.FontFamily]::new('Times New Roman')
$axWord = [Drawing.Drawing2D.GraphicsPath]::new([Drawing.Drawing2D.FillMode]::Winding)
$axWord.AddString('A', $axFont, 0, 150, [Drawing.PointF]::new(0,0), [Drawing.StringFormat]::GenericTypographic)
$axBounds = $axWord.GetBounds()
$axX = $axBounds.Right + 10
$axTop = $axBounds.Top
$axBottom = $axBounds.Bottom
$axWidth = 105
# One coherent serif X, shared by wordmark and icon.
function Add-AxvarisX($axPath, [float]$axLeft, [float]$axBaselineY, [float]$axEm) {
 $axPath.AddString('X', $axFont, 0, $axEm, [Drawing.PointF]::new($axLeft,$axBaselineY), [Drawing.StringFormat]::GenericTypographic)
}
Add-AxvarisX $axWord $axX 0 150
$axWord.AddString('VARIS', $axFont, 0, 150, [Drawing.PointF]::new($axX+$axWidth+12,0), [Drawing.StringFormat]::GenericTypographic)
$axBounds = $axWord.GetBounds()
$axWord.Transform([Drawing.Drawing2D.Matrix]::new(1,0,0,1,12-$axBounds.X,12-$axBounds.Y))
$axBounds = $axWord.GetBounds()
$axMark = [Drawing.Drawing2D.GraphicsPath]::new([Drawing.Drawing2D.FillMode]::Winding)
Add-AxvarisX $axMark 10 0 110
$axMarkBounds=$axMark.GetBounds()
$axMarkScale=72/[Math]::Max($axMarkBounds.Width,$axMarkBounds.Height)
$axMark.Transform([Drawing.Drawing2D.Matrix]::new($axMarkScale,0,0,$axMarkScale,50-($axMarkBounds.X+$axMarkBounds.Width/2)*$axMarkScale,50-($axMarkBounds.Y+$axMarkBounds.Height/2)*$axMarkScale))
function ConvertTo-SvgPath($axPath) {
 $axPoints = $axPath.PathPoints; $axTypes = $axPath.PathTypes; $axD = [Text.StringBuilder]::new()
 function F($v) { $v.ToString('0.###',[Globalization.CultureInfo]::InvariantCulture) }
 for ($axI=0; $axI -lt $axPoints.Length; $axI++) {
  $axKind = $axTypes[$axI] -band 7
  if ($axKind -eq 0) { [void]$axD.Append("M$(F $axPoints[$axI].X) $(F $axPoints[$axI].Y)") }
  elseif ($axKind -eq 1) { [void]$axD.Append("L$(F $axPoints[$axI].X) $(F $axPoints[$axI].Y)") }
  elseif ($axKind -eq 3) { [void]$axD.Append("C$(F $axPoints[$axI].X) $(F $axPoints[$axI].Y) $(F $axPoints[$axI+1].X) $(F $axPoints[$axI+1].Y) $(F $axPoints[$axI+2].X) $(F $axPoints[$axI+2].Y)"); $axI += 2 }
  if (($axTypes[$axI] -band 128) -ne 0) { [void]$axD.Append('Z') }
 }
 return $axD.ToString()
}
function Save-Raster($axPath,$axViewWidth,$axViewHeight,$axSize,$axForeground,$axBackground,$axFile) {
 $axHeight = [int][Math]::Round($axSize*$axViewHeight/$axViewWidth)
 $axBitmap = [Drawing.Bitmap]::new($axSize,$axHeight)
 $axGraphics = [Drawing.Graphics]::FromImage($axBitmap)
 $axGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
 $axGraphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::Half
 $axGraphics.Clear($axBackground)
 $axGraphics.ScaleTransform($axSize/$axViewWidth,$axHeight/$axViewHeight)
 $axBrush = [Drawing.SolidBrush]::new($axForeground)
 $axGraphics.FillPath($axBrush,$axPath)
 $axBitmap.Save($axFile,[Drawing.Imaging.ImageFormat]::Png)
 $axBrush.Dispose(); $axGraphics.Dispose(); $axBitmap.Dispose()
}
$axWordWidth = [Math]::Ceiling($axBounds.Right+12); $axWordHeight = [Math]::Ceiling($axBounds.Bottom+12)
foreach ($axVariant in @(@{Name='light';Color='#eeeeee'},@{Name='dark';Color='#101010'})) {
 foreach ($axAsset in @(@{Name='mark';Path=$axMark;Width=100;Height=100},@{Name='wordmark';Path=$axWord;Width=$axWordWidth;Height=$axWordHeight})) {
  $axSvg = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 '+$axAsset.Width+' '+$axAsset.Height+'" role="img" aria-label="Axvaris"><title>Axvaris</title><path fill="'+$axVariant.Color+'" fill-rule="nonzero" d="'+(ConvertTo-SvgPath $axAsset.Path)+'"/></svg>'
  [IO.File]::WriteAllText("$axOut/axvaris-$($axAsset.Name)-$($axVariant.Name).svg",$axSvg)
  Save-Raster $axAsset.Path $axAsset.Width $axAsset.Height 1024 ([Drawing.ColorTranslator]::FromHtml($axVariant.Color)) ([Drawing.Color]::Transparent) "$axOut/axvaris-$($axAsset.Name)-$($axVariant.Name).png"
 }
}
Save-Raster $axMark 100 100 1024 ([Drawing.ColorTranslator]::FromHtml('#eeeeee')) ([Drawing.ColorTranslator]::FromHtml('#101010')) "$axOut/axvaris-app.png"
$axIcons = [IO.Path]::GetFullPath("$PSScriptRoot/../apps/desktop/src-tauri/icons")
$axSizes = @(16,20,24,32,40,48,64,96,128,192,256)
foreach ($axSize in $axSizes) { Save-Raster $axMark 100 100 $axSize ([Drawing.ColorTranslator]::FromHtml('#eeeeee')) ([Drawing.ColorTranslator]::FromHtml('#101010')) "$axIcons/$($axSize)x$($axSize).png" }
[IO.File]::Copy("$axIcons/256x256.png","$axIcons/128x128@2x.png",$true)
$axStream = [IO.MemoryStream]::new(); $axWriter = [IO.BinaryWriter]::new($axStream)
$axWriter.Write([uint16]0); $axWriter.Write([uint16]1); $axWriter.Write([uint16]$axSizes.Length)
$axOffset = 6+16*$axSizes.Length
foreach ($axSize in $axSizes) {
 $axData = [IO.File]::ReadAllBytes("$axIcons/$($axSize)x$($axSize).png")
 $axWriter.Write([byte]($axSize%256)); $axWriter.Write([byte]($axSize%256)); $axWriter.Write([uint16]0)
 $axWriter.Write([uint16]1); $axWriter.Write([uint16]32); $axWriter.Write([uint32]$axData.Length); $axWriter.Write([uint32]$axOffset)
 $axOffset += $axData.Length
}
foreach ($axSize in $axSizes) { $axWriter.Write([byte[]][IO.File]::ReadAllBytes("$axIcons/$($axSize)x$($axSize).png")) }
[IO.File]::WriteAllBytes("$axIcons/icon.ico",$axStream.ToArray())
$axWriter.Dispose(); $axStream.Dispose(); $axWord.Dispose(); $axMark.Dispose(); $axFont.Dispose()
Write-Output "Axvaris vector marks, PNG exports and multi-resolution Windows icon generated."
