

```shell
git remote add upstream https://github.com/tangency-labs/solidcraft.git
git remote -v
git remote 
```


```shell
# 切到本地 main
git checkout main
# 先从原作者那里拉取最新代码
git fetch upstream
# 同步最新代码
git merge upstream/main


# 合并（Merge）,保留两边的提交记录，并在顶端生成一个新的“合并提交”
git pull origin main --no-rebase

# 变基（Rebase）
git pull origin main --rebase
# 如果你喜欢用变基（追求整洁）(全局默认)
git config --global pull.rebase true
```


## 在同一个本地仓库里（把本地的 main 同步到分支）
```shell
# 切回 main 并更新
# 确保你在 main 分支
git checkout main
# 拉取远程 main 的最新代码
git pull origin main

# 切回你的分支并合并
git checkout dev_learning
git merge main

```

