$ErrorActionPreference = "Stop"
$rand = [System.Random]::new(20261112)

function rnd($min, $max) { $rand.Next($min, $max) }
function rndD($min, $max) { $min + $rand.NextDouble() * ($max - $min) }

# --- MetricValue builders (serde tagged enum: kind + content) ---
function nv($v) { [ordered]@{ kind="Number"; value=$v } }
function bv($b) { [ordered]@{ kind="Binary"; value=$b } }
function pv($v) { [ordered]@{ kind="Percentage"; value=$v } }
function mv($v) { [ordered]@{ kind="Money"; value=[int64]$v } }
function dv($s) { [ordered]@{ kind="Deadline"; value=$s } }
function dtv($s) { [ordered]@{ kind="Date"; value=$s } }
function tv($s) { [ordered]@{ kind="Text"; value=$s } }
function nvl() { [ordered]@{ kind="Null"; value=$null } }

$metricDefsList = @(
    [ordered]@{ id="num"; name="Выработка"; kind="Number"; unit="шт." },
    [ordered]@{ id="dem"; name="Потребность"; kind="Number"; unit="шт." },
    [ordered]@{ id="pct"; name="Загрузка"; kind="Percentage"; unit="%" },
    [ordered]@{ id="bin"; name="Готовность"; kind="Binary"; unit="" },
    [ordered]@{ id="mon"; name="Бюджет"; kind="Money"; unit="₽" },
    [ordered]@{ id="dl"; name="Срок"; kind="Deadline"; unit="дд.мм.гггг чч:мм" },
    [ordered]@{ id="dt"; name="Дата"; kind="Date"; unit="дд.мм.гггг" },
    [ordered]@{ id="txt"; name="Примечание"; kind="Text"; unit="" }
)

$objects = New-Object System.Collections.ArrayList

function add-obj($id, $name, $kind, $x, $y, $parent, $reqs, $outs) {
    $o = [ordered]@{
        id=$id; name=$name; kind=$kind; at=@($x,$y); parent=$parent;
        requests=$reqs; outputs=$outs
    }
    $objects.Add($o) | Out-Null
}

function m-entry($id, $md, $dir, $value, $required, $max, $fact) {
    [ordered]@{
        id=$id; metric_def=$md; dir=$dir; value=$value; required=$required; max=$max; fact=$fact
    }
}

$gridPitch = 36
$gridCols  = 32
$gridIdx   = 0
function next-grid {
    $x = ($script:gridIdx % $script:gridCols) * $script:gridPitch
    $y = [math]::Floor($script:gridIdx / $script:gridCols) * $script:gridPitch
    $script:gridIdx++
    return @($x,$y)
}

# Standard metric sets.
$zero = 0
function dept-metrics($V, $R) {
    $outs = @(m-entry "o1" "num" "output" (nv $V) (nv $zero) $null (nvl))
    $reqs = @(m-entry "i1" "dem" "request" (nvl) (nv $R) $null (nvl))
    return @($reqs, $outs)
}
function proc-metrics($V, $R) {
    $outs = @(m-entry "o1" "num" "output" (nv $V) (nv $zero) $null (nvl))
    $reqs = @(m-entry "i1" "dem" "request" (nvl) (nv $R) $null (nvl))
    return @($reqs, $outs)
}

# Special hierarchy subtree (reserved region x>=2200), added first so parents exist.
# dep000/dep001/dep002 are departments #0..#2; proc000/proc001/proc002 are processes #0..#2.
$dV = [math]::Round((rndD 40 160), 2); $dR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = dept-metrics $dV $dR
add-obj "dep000" "Центральный офис" "Department" 2200 0 $null $reqs $outs
$dV = [math]::Round((rndD 40 160), 2); $dR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = dept-metrics $dV $dR
add-obj "dep001" "Цех А"             "Department" 2200 40 "dep000" $reqs $outs
$dV = [math]::Round((rndD 40 160), 2); $dR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = dept-metrics $dV $dR
add-obj "dep002" "Цех Б"             "Department" 2200 80 "dep000" $reqs $outs
$pV = [math]::Round((rndD 40 160), 2); $pR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = proc-metrics $pV $pR
add-obj "proc000" "Процесс 000"      "Process"      2240 0  "dep000" $reqs $outs
$pV = [math]::Round((rndD 40 160), 2); $pR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = proc-metrics $pV $pR
add-obj "proc001" "Процесс 001"      "Process"      2240 40 "dep001" $reqs $outs
$pV = [math]::Round((rndD 40 160), 2); $pR = [math]::Round((rndD 40 160), 2)
$reqs,$outs = proc-metrics $pV $pR
add-obj "proc002" "Процесс 002"      "Process"      2240 80 "dep002" $reqs $outs

# ---- Departments (150 total: dep000..dep149) ----
for ($i = 3; $i -lt 150; $i++) {
    $id = ("dep{0:D3}" -f $i)
    $name = "Отдел {0:D3}" -f $i
    $xy = next-grid
    $V = [math]::Round((rndD 40 160), 2)
    $R = [math]::Round((rndD 40 160), 2)
    $outs = @(m-entry "o1" "num" "output" (nv $V) (nv $zero) $null (nvl))
    $reqs = @(m-entry "i1" "dem" "request" (nvl) (nv $R) $null (nvl))
    add-obj $id $name "Department" $xy[0] $xy[1] $null $reqs $outs
}

# ---- Processes (750 total: proc000..proc749) ----
# proc000..proc002 already added as special. Add proc003..proc749 on the grid.
for ($i = 3; $i -lt 750; $i++) {
    $id = ("proc{0:D3}" -f $i)
    $name = "Процесс {0:D3}" -f $i
    $xy = next-grid
    $V = [math]::Round((rndD 40 160), 2)
    $R = [math]::Round((rndD 40 160), 2)
    # balanced cases for clean Ok examples
    if ($i % 50 -eq 0) { $V = 100.0; $R = 100.0 }
    $outs = @(m-entry "o1" "num" "output" (nv $V) (nv $zero) $null (nvl))
    $reqs = @(m-entry "i1" "dem" "request" (nvl) (nv $R) $null (nvl))
    add-obj $id $name "Process" $xy[0] $xy[1] $null $reqs $outs
}

# ---- Edges (связи) ----
$edges = New-Object System.Collections.ArrayList

# Кольцевая цепь процессов: proc[i].o1 -> proc[(i+1)%750].i1 (через одно ~75% связано)
for ($i = 0; $i -lt 750; $i++) {
    if ($i % 4 -eq 0) { continue }  # ~25% без связи => Unconnected
    $from = ("proc{0:D3}" -f $i)
    $to   = ("proc{0:D3}" -f (($i + 1) % 750))
    $edges.Add([ordered]@{ from_object=$from; from_entry="o1"; to_object=$to; to_entry="i1"; throughput=100.0 }) | Out-Null
}

# Отделы -> свободные входы процессов (proc[i].i1 без кольцевой связи, т.е. i%4==1)
for ($k = 0; $k -lt 38; $k++) {
    $d = ("dep{0:D3}" -f $k)
    $m = ("proc{0:D3}" -f (4*$k + 1))
    $edges.Add([ordered]@{ from_object=$d; from_entry="o1"; to_object=$m; to_entry="i1"; throughput=100.0 }) | Out-Null
}

# Цепочка внутри иерархии отдела: proc000.o1 -> proc001.i1 -> proc002.i1
$edges.Add([ordered]@{ from_object="proc000"; from_entry="o1"; to_object="proc001"; to_entry="i1"; throughput=100.0 }) | Out-Null
$edges.Add([ordered]@{ from_object="proc001"; from_entry="o1"; to_object="proc002"; to_entry="i1"; throughput=100.0 }) | Out-Null

# Force several explicit Deficit / Surplus / Ok cases on connected process outputs
# (перезаписываем значения уже созданных записей — они лежат в документе как value/required)
# Делаем это через правку уже добавленных объектов: проще — выставим при генерации выше.
# Здесь дополнительно гарантируем явные случаи через перезапись value/required в уже добавленных объектах.

# Сборка документа
$doc = [ordered]@{
    format="polygram/2"
    name="Сtresstest 1000 объектов"
    metric_defs=$metricDefsList
    objects=$objects
    edges=$edges
}

$json = $doc | ConvertTo-Json -Depth 100
$outPath = "C:\Dev\Polygram\examples\stress_1000.pg"
$utf8 = New-Object System.IO.StreamWriter($outPath, $false, (New-Object System.Text.UTF8Encoding $false))
$utf8.Write($json)
$utf8.Close()

Write-Host "Written $outPath"
Write-Host ("Objects: " + $objects.Count)
Write-Host ("Edges: " + $edges.Count)
