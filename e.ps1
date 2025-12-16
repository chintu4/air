param(
    [string]$cmd,
    [string]$value
)

switch ($cmd) {
    "push" {
        git add .
        git commit -m "autocommit"
        git push
    }

    "pull" {
        git pull origin master
    }

    "pr" {
        if (-not $value) {
            Write-Error "PR number required"
            exit 1
        }
        echo "looking at pr $value"
        gh pr ready $value
        gh pr merge $value --merge --delete-branch
    }

    default {
        Write-Error "Unknown command"
    }
}
