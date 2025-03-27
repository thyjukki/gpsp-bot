package platforms

import (
	"time"

	"github.com/napuu/gpsp-bot/internal/chain"
	"github.com/napuu/gpsp-bot/internal/config"
	"github.com/napuu/gpsp-bot/internal/handlers"

	tele "gopkg.in/telebot.v4"
)

func wrapTeleHandler(bot *tele.Bot, chain *chain.HandlerChain) func(c tele.Context) error {
	return func(c tele.Context) error {
		chain.Process(&handlers.Context{TelebotContext: c, Telebot: bot, Service: handlers.Telegram})
		return nil
	}
}

func TelebotCompatibleVisibleCommands() []tele.Command {
	commands := make([]tele.Command, 0, len(config.EnabledFeatures()))
	for action := range config.EnabledFeatures() {
		commands = append(commands, tele.Command{
			Text:        string(action),
			Description: string(handlers.ActionMap[handlers.Action(action)]),
		})
	}
	return commands
}

func RunTelegramBot() {
	bot := getTelegramBot()
	chain := chain.NewChainOfResponsibility()

	bot.SetCommands(TelebotCompatibleVisibleCommands())

	bot.Handle(tele.OnText, wrapTeleHandler(bot, chain))

	go bot.Start()
}

func getTelegramBot() *tele.Bot {
	pref := tele.Settings{
		Token:     config.FromEnv().TELEGRAM_TOKEN,
		ParseMode: tele.ModeHTML,
		Poller: &tele.LongPoller{
			Timeout: 10 * time.Second,
			AllowedUpdates: []string{
				"message",
				// TODO - take this into use
				// "message_reaction",
			},
		},
	}

	b, err := tele.NewBot(pref)
	if err != nil {
		panic(err)
	}

	return b
}
