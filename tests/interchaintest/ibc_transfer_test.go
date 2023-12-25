package interchaintest

import (
	"context"
	"fmt"
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

type AssetInfos struct {
	NativeAssetDenom string `json:"native_asset_denom"`
	LsAssetDenom     string `json:"ls_asset_denom"`
}

type ContractInstantiateMsg struct {
	Assets AssetInfos `json:"assets"`
}

type QueryLsConfigMsg struct {
	LsConfig struct{} `json:"ls_config"`
}

type QueryStakedLiquidityMsg struct {
	GetStakedLiquidity struct{} `json:"get_staked_liquidity"`
}

type QueryAssetsMsg struct {
	Assets struct{} `json:"assets"`
}

type Active struct {
	Active bool `json:"active"`
}

type QueryLsConfigResp struct {
	Data Active `json:"data"`
}

type StakedLAmountNative struct {
	StakedLAmountNative string `json:"staked_amount_native"`
}

type QueryStakedLiquidityResp struct {
	Data StakedLAmountNative `json:"data"`
}

type QueryAssetsResp struct {
	Data AssetInfos `json:"data"`
}

var (
	queryLsConfigMsg             QueryLsConfigMsg
	queryLsConfigResp            QueryLsConfigResp
	queryStakedLiquidityMsg      QueryStakedLiquidityMsg
	queryStakedLiquidityResp     QueryStakedLiquidityResp
	queryAssetsMsg               QueryAssetsMsg
	queryAssetsResp              QueryAssetsResp
	icaLiquidStakingContractAddr string
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
		ibcPath  = "ibc-path"
		stkDenom = "stk/uatom"
		wasmPath = "../../artifacts/ica_liquid_staking.wasm"
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

	// Get persistence admin account
	persistenceAdminMnemonic := "tone cause tribe this switch near host damage idle fragile antique tail soda alien depth write wool they rapid unfold body scan pledge soft"
	persistenceAdmin, err := interchaintest.GetAndFundTestUserWithMnemonic(ctx, t.Name(), persistenceAdminMnemonic, genesisWalletAmount, persistenceChain)
	require.NoError(t, err)

	persistenceAdminAddr := persistenceAdmin.FormattedAddress()

	err = testutil.WaitForBlocks(ctx, 10, persistenceChain, gaiaChain)
	require.NoError(t, err, "failed to wait for blocks")

	// Get original account balances
	persistenceOrigBal, err := persistenceChain.GetBalance(ctx, persistenceUserAddr, persistenceChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, math.NewInt(genesisWalletAmount), persistenceOrigBal)

	persistenceAdminOrigBal, err := persistenceChain.GetBalance(ctx, persistenceAdminAddr, persistenceChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, math.NewInt(genesisWalletAmount), persistenceAdminOrigBal)

	gaiaOrigBal, err := gaiaChain.GetBalance(ctx, gaiaUserAddr, gaiaChain.Config().Denom)
	require.NoError(t, err)
	require.Equal(t, math.NewInt(genesisWalletAmount), gaiaOrigBal)

	// Get Channel ID
	gaiaChannelInfo, err := r.GetChannels(ctx, eRep, gaiaChain.Config().ChainID)
	require.NoError(t, err)
	gaiaChannelID := gaiaChannelInfo[0].ChannelID

	channel, err := ibc.GetTransferChannel(ctx, r, eRep, persistenceChain.Config().ChainID, gaiaChain.Config().ChainID)
	require.NoError(t, err)

	// Get the IBC denom for uatom on Persistence
	gaiaTokenDenom := transfertypes.GetPrefixedDenom(channel.Counterparty.PortID, channel.Counterparty.ChannelID, gaiaChain.Config().Denom)
	gaiaIBCDenom := transfertypes.ParseDenomTrace(gaiaTokenDenom).IBCDenom()

	t.Run("register host chain", func(t *testing.T) {

		cmd := []string{"persistenceCore", "tx", "liquidstakeibc", "register-host-chain",
			channel.ConnectionHops[0], channel.ChannelID, channel.PortID,
			"0.00", "0.05", "0.00", "0.005", gaiaChain.Config().Denom, "1", "4", "2",
			"--from", persistenceAdmin.KeyName(),
			"--gas", "auto",
			"--gas-adjustment", `1.3`,
			"--output", "json",
			"--chain-id", persistenceChain.Config().ChainID,
			"--node", persistenceChain.GetRPCAddress(),
			"--home", persistenceChain.HomeDir(),
			"--keyring-backend", "test",
			"-y",
		}

		_, _, err = persistenceChain.Exec(ctx, cmd, nil)
		require.NoError(t, err, "failed to register host chain on persistence")

		err = testutil.WaitForBlocks(ctx, 5, persistenceChain)
		require.NoError(t, err, "failed to wait for blocks")
	})

	t.Run("update host chain", func(t *testing.T) {

		cmd := []string{"persistenceCore", "tx", "liquidstakeibc", "update-host-chain",
			gaiaChain.Config().ChainID, `[{"key": "active","value": "true"}]`,
			"--from", persistenceAdmin.KeyName(),
			"--gas", "auto",
			"--gas-adjustment", `1.3`,
			"--output", "json",
			"--chain-id", persistenceChain.Config().ChainID,
			"--node", persistenceChain.GetRPCAddress(),
			"--home", persistenceChain.HomeDir(),
			"--keyring-backend", "test",
			"-y",
		}

		_, _, err = persistenceChain.Exec(ctx, cmd, nil)
		require.NoError(t, err, "failed to update host chain on persistence")

		err = testutil.WaitForBlocks(ctx, 5, persistenceChain)
		require.NoError(t, err, "failed to wait for blocks")
	})

	t.Run("deploy liquid stake contract", func(t *testing.T) {

		// Store ica_liquid_staking.wasm contract
		icaLiquidStakingCodeId, err := persistenceChain.StoreContract(ctx, persistenceUser.KeyName(), wasmPath)
		require.NoError(t, err)

		// Instantiate ica_liquid_staking.wasm contract
		initMsg := fmt.Sprintf(`{"assets": {"native_asset_denom": "%s", "ls_asset_denom": "%s"}}`, gaiaIBCDenom, stkDenom)
		icaLiquidStakingContractAddr, err = persistenceChain.InstantiateContract(
			ctx, persistenceUser.KeyName(), icaLiquidStakingCodeId, initMsg, true)
		require.NoError(t, err)

		// wait for 2 blocks to pass
		err = testutil.WaitForBlocks(ctx, 2, persistenceChain)
		require.NoError(t, err)

		// Query ica_liquid_staking.wasm contract
		err = persistenceChain.QueryContract(ctx, icaLiquidStakingContractAddr, queryLsConfigMsg, &queryLsConfigResp)
		require.NoError(t, err)
		require.Equal(t, true, queryLsConfigResp.Data.Active)

		// Query ica_liquid_staking.wasm contract
		err = persistenceChain.QueryContract(ctx, icaLiquidStakingContractAddr, queryStakedLiquidityMsg, &queryStakedLiquidityResp)
		require.NoError(t, err)
		require.Equal(t, "0", queryStakedLiquidityResp.Data.StakedLAmountNative)

		// Query ica_liquid_staking.wasm contract
		err = persistenceChain.QueryContract(ctx, icaLiquidStakingContractAddr, queryAssetsMsg, &queryAssetsResp)
		require.NoError(t, err)
		require.Equal(t, gaiaIBCDenom, queryAssetsResp.Data.NativeAssetDenom)
		require.Equal(t, stkDenom, queryAssetsResp.Data.LsAssetDenom)
	})

	t.Run("ibc transfer atom with memo", func(t *testing.T) {

		// Note the height before the transfer
		gaiaHeight, err := gaiaChain.Height(ctx)
		require.NoError(t, err)

		// Compose an IBC transfer and send from Gaia -> Persistence
		var transferAmount = math.NewInt(1_000)
		transfer := ibc.WalletAmount{
			Address: icaLiquidStakingContractAddr,
			Denom:   gaiaChain.Config().Denom,
			Amount:  transferAmount,
		}
		executeMsg := fmt.Sprintf(`{"liquid_stake":{"receiver":"%s"}}`, persistenceUserAddr)
		memo := fmt.Sprintf(`{"wasm":{"contract":"%s","msg":%s}}`, icaLiquidStakingContractAddr, executeMsg)
		transferTx, err := gaiaChain.SendIBCTransfer(ctx, gaiaChannelID, gaiaUser.KeyName(), transfer, ibc.TransferOptions{
			Timeout: &ibc.IBCTimeout{
				Height:      0,
				NanoSeconds: 0,
			},
			Memo: memo,
		})
		require.NoError(t, err)
		require.NoError(t, transferTx.Validate())

		// relay MsgRecvPacket to persistence, then MsgAcknowledgement back to gaia
		require.NoError(t, r.Flush(ctx, eRep, ibcPath, gaiaChannelID))

		// Poll for the ack to know the transfer was successful
		_, err = testutil.PollForAck(ctx, gaiaChain, gaiaHeight, gaiaHeight+25, transferTx.Packet)
		require.NoError(t, err)

		// wait for 2 blocks to pass
		err = testutil.WaitForBlocks(ctx, 2, persistenceChain, gaiaChain)
		require.NoError(t, err)

		// Test source wallet has decreased funds
		gaiaUpdateBal, err := gaiaChain.GetBalance(ctx, gaiaUserAddr, gaiaChain.Config().Denom)
		require.NoError(t, err)
		require.Equal(t, gaiaOrigBal.Sub(transferAmount), gaiaUpdateBal)

		// Test destination wallet has no ibc funds
		persistenceUpdateBal, err := persistenceChain.GetBalance(ctx, persistenceUserAddr, gaiaIBCDenom)
		require.NoError(t, err)
		require.Equal(t, math.ZeroInt(), persistenceUpdateBal)

		persistenceUpdateBal, err = persistenceChain.GetBalance(ctx, icaLiquidStakingContractAddr, gaiaIBCDenom)
		require.NoError(t, err)
		require.Equal(t, math.ZeroInt(), persistenceUpdateBal)

		// Test destination wallet has increased stk funds
		persistenceUpdateBal, err = persistenceChain.GetBalance(ctx, persistenceUserAddr, stkDenom)
		require.NoError(t, err)
		require.Equal(t, transferAmount, persistenceUpdateBal)
	})
}
