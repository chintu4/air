if ($args -eq "push"){
    git add .
    git commit -m "autocommit"
    git push
}
if ($args -eq "pull"){
    git add .
    git commit -m "pull commit"
    git pull origin master
}