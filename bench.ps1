# MemScan Benchmark Runner (PowerShell)
# Usage: .\bench.ps1 [command]

param(
    [Parameter(Position=0)]
    [string]$Command = "all",
    
    [Parameter(Position=1)]
    [string]$Argument = ""
)

function Show-Help {
    Write-Host @"
MemScan Benchmark Runner

Usage: .\bench.ps1 [command]

Commands:
    all                 Run all benchmarks (default)
    pattern             Run pattern search benchmarks only
    hex                 Run hex parsing benchmarks only
    baseline [name]     Save current results as baseline
    compare [baseline]  Compare against saved baseline
    quick               Run quick benchmarks (reduced sample size)
    clean               Remove benchmark cache and results
    report              Open HTML report in browser
    help                Show this help message

Examples:
    .\bench.ps1                     # Run all benchmarks
    .\bench.ps1 baseline main       # Save baseline as 'main'
    .\bench.ps1 compare main        # Compare against 'main' baseline
    .\bench.ps1 pattern             # Run only pattern search benchmarks
    .\bench.ps1 report              # Open results in browser

Environment Variables:
    SAMPLE_SIZE         Override sample size (default: 100)
    MEASUREMENT_TIME    Override measurement time in seconds (default: 5)
"@
}

function Run-All {
    Write-Host "Running all benchmarks..." -ForegroundColor Cyan
    cargo bench --no-fail-fast
}

function Run-Pattern {
    Write-Host "Running pattern search benchmarks..." -ForegroundColor Cyan
    cargo bench --bench pattern_search --no-fail-fast
}

function Run-Hex {
    Write-Host "Running hex parsing benchmarks..." -ForegroundColor Cyan
    cargo bench --bench hex_parsing --no-fail-fast
}

function Save-Baseline {
    param([string]$Name = "baseline")
    Write-Host "Saving baseline as '$Name'..." -ForegroundColor Cyan
    cargo bench --no-fail-fast -- --save-baseline $Name
    Write-Host "Baseline '$Name' saved successfully" -ForegroundColor Green
}

function Compare-Baseline {
    param([string]$Name = "baseline")
    Write-Host "Comparing against baseline '$Name'..." -ForegroundColor Cyan
    cargo bench --no-fail-fast -- --baseline $Name
}

function Run-Quick {
    Write-Host "Running quick benchmarks (reduced samples)..." -ForegroundColor Cyan
    $env:CRITERION_SAMPLE_SIZE = "20"
    cargo bench --no-fail-fast
}

function Clean-Benchmarks {
    Write-Host "Cleaning benchmark results..." -ForegroundColor Cyan
    if (Test-Path "target/criterion") {
        Remove-Item -Recurse -Force "target/criterion"
    }
    Write-Host "Benchmark results cleaned" -ForegroundColor Green
}

function Open-Report {
    $reportPath = "target\criterion\report\index.html"
    if (-not (Test-Path $reportPath)) {
        Write-Host "Error: No benchmark report found. Run benchmarks first." -ForegroundColor Red
        exit 1
    }
    
    Write-Host "Opening benchmark report..." -ForegroundColor Cyan
    Start-Process $reportPath
}

# Main command dispatcher
switch ($Command.ToLower()) {
    "all" {
        Run-All
    }
    "pattern" {
        Run-Pattern
    }
    "hex" {
        Run-Hex
    }
    "baseline" {
        if ([string]::IsNullOrEmpty($Argument)) {
            $Argument = "baseline"
        }
        Save-Baseline -Name $Argument
    }
    "compare" {
        if ([string]::IsNullOrEmpty($Argument)) {
            $Argument = "baseline"
        }
        Compare-Baseline -Name $Argument
    }
    "quick" {
        Run-Quick
    }
    "clean" {
        Clean-Benchmarks
    }
    "report" {
        Open-Report
    }
    "help" {
        Show-Help
    }
    default {
        Write-Host "Error: Unknown command '$Command'" -ForegroundColor Red
        Write-Host ""
        Show-Help
        exit 1
    }
}
