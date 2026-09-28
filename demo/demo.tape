Output demo/demo.gif

Set Shell "bash"
Set FontSize 20
Set Width 1300
Set Height 720
Set Padding 28
Set Margin 24
Set MarginFill "#6B50FF"
Set BorderRadius 10
Set WindowBar Colorful
Set Theme "Catppuccin Mocha"
Set TypingSpeed 30ms

Hide
Sleep 1s
Type `PS1='\[\e[38;2;203;166;247m\]$\[\e[0m\] '`
Enter
Type `export PATH="$HOME/.local/bin:$PATH" && cd "$(mktemp -d)" && export JEVPIPE_CONFIG="$PWD/config.toml" && jevpipe config set provider typesafe`
Enter
Type `curl -sL https://raw.githubusercontent.com/PolyAI-LDN/task-specific-datasets/master/banking_data/test.csv | python3 -c 'import csv,sys; [print(r["text"].strip()) for r in csv.DictReader(sys.stdin)]' | shuf -n 1000 --random-source=<(yes) > inbox.txt`
Enter
Type `echo '{"team": {"type": "choice", "instructions": "Which team should answer this?", "criteria": {"cards": "card delivery, activation and payments", "transfers": "transfers, top-ups and exchange", "account": "identity, login and account details", "fraud": "fraud, stolen or compromised cards"}}}' | jq . > team.json && clear`
Enter
Wait+Line@60s /\$$/
Show

Sleep 500ms
Type "# 1,000 messages from a bank's support inbox"
Sleep 600ms
Enter
Type "head -3 inbox.txt"
Sleep 400ms
Enter
Sleep 2s

Type `jevpipe filter "Is the customer threatening to leave the bank?" inbox.txt`
Sleep 500ms
Enter
Wait+Line@120s /\$$/
Sleep 1s
Screenshot demo/filter.png
Sleep 2s
Ctrl+L
Sleep 500ms

Type "cat team.json"
Sleep 400ms
Enter
Sleep 3s

Type "jevpipe map -f team.json inbox.txt | jq -r .answers.team.choice | sort | uniq -c"
Sleep 500ms
Enter
Sleep 2s
Hide
Wait+Line@120s /\$$/
Show
Sleep 1s
Screenshot demo/map.png
Sleep 3s
