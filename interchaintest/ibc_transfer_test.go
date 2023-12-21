package interchaintest

import (
	"context"
	"testing"

	"cosmossdk.io/math"
	transfertypes "github.com/cosmos/ibc-go/v7/modules/apps/transfer/types"
	"github.com/strangelove-ventures/interchaintest/v7"
	"github.com/strangelove-ventures/interchaintest/v7/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v7/ibc"
	interchaintestrelayer "github.com/strangelove-ventures/interchaintest/v7/relayer"
	"github.com/strangelove-ventures/interchaintest/v7/testreporter"
	"github.com/strangelove-ventures/interchaintest/v7/testutil"
	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zaptest"
)

// TestPersistenceGaiaIBCTransfer spins up a Persistence and Gaia network, initializes an IBC connection between them,
// and sends an ICS20 token transfer from Gaia->Persistence.
func TestPersistenceGaiaIBCTransfer(t *testing.T) {
	if testing.Short() {
		t.Skip()
	}

	t.Parallel()

	// Create chain factory with Persistence and Gaia
	numVals := 1
	numFullNodes := 1

	cf := interchaintest.NewBuiltinChainFactory(zaptest.NewLogger(t), []*interchaintest.ChainSpec{
		{
			Name:          "persistence",
			ChainConfig:   persistenceChainConfig(),
			NumValidators: &numVals,
			NumFullNodes:  &numFullNodes,
		},
		{
			Name: "gaia",
			ChainConfig: ibc.ChainConfig{
				GasPrices: "0.0uatom",
			},
			Version:       "v9.1.0",
			NumValidators: &numVals,
			NumFullNodes:  &numFullNodes,
		},
	})

	const (
		ibcPath = "ibc-path"
	)

	// Get chains from the chain factory
	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)

	client, network := interchaintest.DockerSetup(t)

	persistenceChain, gaiaChain := chains[0].(*cosmos.CosmosChain), chains[1].(*cosmos.CosmosChain)

	relayerType, relayerName := ibc.CosmosRly, "relay"

	// Get a relayer instance
	rf := interchaintest.NewBuiltinRelayerFactory(
		relayerType,
		zaptest.NewLogger(t),
		interchaintestrelayer.CustomDockerImage(IBCRelayerImage, IBCRelayerVersion, "100:1000"),
		interchaintestrelayer.StartupFlags("--processor", "events", "--block-history", "100"),
	)

	r := rf.Build(t, client, network)

	ic := interchaintest.NewInterchain().
		AddChain(persistenceChain).
		AddChain(gaiaChain).
		AddRelayer(r, relayerName).
		AddLink(interchaintest.InterchainLink{
			Chain1:  persistenceChain,
			Chain2:  gaiaChain,
			Relayer: r,
			Path:    ibcPath,
		})

	ctx := context.Background()

	rep := testreporter.NewNopReporter()
	eRep := rep.RelayerExecReporter(t)

	require.NoError(t, ic.Build(ctx, eRep, interchaintest.InterchainBuildOptions{
		TestName:          t.Name(),
		Client:            client,
		NetworkID:         network,
		BlockDatabaseFile: interchaintest.DefaultBlockDatabaseFilepath(),
		SkipPathCreation:  false,
	}))
	t.Cleanup(func() {
		_ = ic.Close()
	})

	// Create some user accounts on both chains
	users := interchaintest.GetAndFundTestUsers(t, ctx, t.Name(), genesisWalletAmount, persistenceChain, gaiaChain)

	// Wait a few blocks for relayer to start and for user accounts to be created
	err = testutil.WaitForBlocks(ctx, 5, persistenceChain, gaiaChain)
	require.NoError(t, err)

	// Get our Bech32 encoded user addresses
	persistenceUser, gaiaUser := users[0], users[1]

	persistenceUserAddr := persistenceUser.FormattedAddress()
	gaiaUserAddr := gaiaUser.FormattedAddress()

	// Get original account balances
	persistenceOrigBal, err := persistenceChain.GetBalance(ctx, persistenceUserAddr, persistenceChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, math.NewInt(genesisWalletAmount), persistenceOrigBal)

	gaiaOrigBal, err := gaiaChain.GetBalance(ctx, gaiaUserAddr, gaiaChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, math.NewInt(genesisWalletAmount), gaiaOrigBal)

	// Get Channel ID
	gaiaChannelInfo, err := r.GetChannels(ctx, eRep, gaiaChain.Config().ChainID)
	require.NoError(t, err)
	gaiaChannelID := gaiaChannelInfo[0].ChannelID

	var transferAmount = math.NewInt(1_000)

	channel, err := ibc.GetTransferChannel(ctx, r, eRep, persistenceChain.Config().ChainID, gaiaChain.Config().ChainID)
	require.NoError(t, err)

	// Get the IBC denom for uatom on Persistence
	gaiaTokenDenom := transfertypes.GetPrefixedDenom(channel.Counterparty.PortID, channel.Counterparty.ChannelID, gaiaChain.Config().Denom)
	gaiaIBCDenom := transfertypes.ParseDenomTrace(gaiaTokenDenom).IBCDenom()

	// Compose an IBC transfer and send from Gaia -> Persistence
	transfer := ibc.WalletAmount{
		Address: persistenceUserAddr,
		Denom:   gaiaChain.Config().Denom,
		Amount:  transferAmount,
	}

	gaiaHeight, err := gaiaChain.Height(ctx)
	require.NoError(t, err)

	transferTx, err := gaiaChain.SendIBCTransfer(ctx, gaiaChannelID, gaiaUserAddr, transfer, ibc.TransferOptions{})
	require.NoError(t, err)
	require.NoError(t, transferTx.Validate())

	// relay MsgRecvPacket to persistence, then MsgAcknowledgement back to gaia
	require.NoError(t, r.Flush(ctx, eRep, ibcPath, gaiaChannelID))

	// Poll for the ack to know the transfer was successful
	_, err = testutil.PollForAck(ctx, gaiaChain, gaiaHeight, gaiaHeight+25, transferTx.Packet)
	require.NoError(t, err)

	// Test destination wallet has increased funds
	persistenceUpdateBal, err := persistenceChain.GetBalance(ctx, persistenceUserAddr, gaiaIBCDenom)
	require.NoError(t, err)
	require.Equal(t, transferAmount, persistenceUpdateBal)

	// Test source wallet has decreased funds
	gaiaUpdateBal, err := gaiaChain.GetBalance(ctx, gaiaUserAddr, gaiaChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, gaiaOrigBal.Sub(transferAmount), gaiaUpdateBal)
}
