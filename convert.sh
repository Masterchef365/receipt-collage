for x in images/*; do
    convert $x -dither FloydSteinberg -remap pattern:gray50 $x
done
