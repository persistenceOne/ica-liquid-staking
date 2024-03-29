package interchaintest

import (
	"context"
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
	"go.uber.org/zap/zaptest"

	wasmtypes "github.com/CosmWasm/wasmd/x/wasm/types"
	testutil "github.com/cosmos/cosmos-sdk/types/module/testutil"
	ibclocalhost "github.com/cosmos/ibc-go/v7/modules/light-clients/09-localhost"
	liquidstaketypes "github.com/persistenceOne/pstake-native/v2/x/liquidstake/types"
	liquidstakeibctypes "github.com/persistenceOne/pstake-native/v2/x/liquidstakeibc/types"
	interchaintest "github.com/strangelove-ventures/interchaintest/v7"
	"github.com/strangelove-ventures/interchaintest/v7/chain/cosmos"
	"github.com/strangelove-ventures/interchaintest/v7/ibc"
	"github.com/strangelove-ventures/interchaintest/v7/testreporter"

	"github.com/persistenceOne/ica-liquid-staking/interchaintest/helpers"
)

var (
	PersistenceE2ERepo = "persistenceone/persistencecore"
	IBCRelayerImage    = "ghcr.io/cosmos/relayer"
	IBCRelayerVersion  = "main"

	PersistenceCoreImage = ibc.DockerImage{
		Repository: "persistenceone/persistencecore",
		Version:    "v11.2.0",
		UidGid:     "1025:1025",
	}

	defaultGenesisOverridesKV = []cosmos.GenesisKV{
		{
			Key:   "app_state.gov.params.voting_period",
			Value: "15s",
		},
		{
			Key:   "app_state.gov.params.max_deposit_period",
			Value: "10s",
		},
		{
			Key:   "app_state.gov.params.min_deposit.0.denom",
			Value: helpers.PersistenceBondDenom,
		},
		{
			Key:   "app_state.builder.params.reserve_fee.denom",
			Value: helpers.PersistenceBondDenom,
		},
		{
			Key:   "app_state.builder.params.min_bid_increment.denom",
			Value: helpers.PersistenceBondDenom,
		},
		{
			Key:   "app_state.wasm.params.code_upload_access.permission",
			Value: "Everybody",
		},
		{
			Key:   "app_state.wasm.params.instantiate_default_permission",
			Value: "Everybody",
		},
		{
			Key: "app_state.interchainaccounts.host_genesis_state.params.allow_messages",
			Value: []string{
				"/cosmos.bank.v1beta1.MsgSend",
				"/cosmos.bank.v1beta1.MsgMultiSend",
				"/cosmos.staking.v1beta1.MsgDelegate",
				"/cosmos.staking.v1beta1.MsgUndelegate",
				"/cosmos.staking.v1beta1.MsgBeginRedelegate",
				"/cosmos.staking.v1beta1.MsgRedeemTokensforShares",
				"/cosmos.staking.v1beta1.MsgTokenizeShares",
				"/cosmos.distribution.v1beta1.MsgWithdrawDelegatorReward",
				"/cosmos.distribution.v1beta1.MsgSetWithdrawAddress",
				"/ibc.applications.transfer.v1.MsgTransfer",
			},
		},
		{
			Key:   "app_state.liquidstakeibc.params.admin_address",
			Value: "persistence1u20df3trc2c2zdhm8qvh2hdjx9ewh00spalt70", // admin
		},
	}

	genesisWalletAmount = int64(10_000_000)
)

// persistenceEncoding registers the persistenceCore specific module codecs so that the associated types and msgs
// will be supported when writing to the blocksdb sqlite database.
func persistenceEncoding() *testutil.TestEncodingConfig {
	cfg := cosmos.DefaultEncoding()

	// register custom types
	ibclocalhost.RegisterInterfaces(cfg.InterfaceRegistry)
	wasmtypes.RegisterInterfaces(cfg.InterfaceRegistry)
	liquidstaketypes.RegisterInterfaces(cfg.InterfaceRegistry)
	liquidstakeibctypes.RegisterInterfaces(cfg.InterfaceRegistry)

	return &cfg
}

// persistenceChainConfig returns dynamic config for persistence chains, allowing to inject genesis overrides
func persistenceChainConfig(
	genesisOverrides ...cosmos.GenesisKV,
) ibc.ChainConfig {
	if len(genesisOverrides) == 0 {
		genesisOverrides = defaultGenesisOverridesKV
	}

	config := ibc.ChainConfig{
		Type:                   "cosmos",
		Name:                   "persistence",
		ChainID:                "ictest-core-1",
		Bin:                    "persistenceCore",
		Bech32Prefix:           "persistence",
		Denom:                  helpers.PersistenceBondDenom,
		CoinType:               fmt.Sprintf("%d", helpers.PersistenceCoinType),
		GasPrices:              fmt.Sprintf("0%s", helpers.PersistenceBondDenom),
		GasAdjustment:          1.5,
		TrustingPeriod:         "112h",
		NoHostMount:            false,
		ConfigFileOverrides:    nil,
		EncodingConfig:         persistenceEncoding(),
		UsingNewGenesisCommand: true,
		ModifyGenesis:          cosmos.ModifyGenesis(genesisOverrides),

		Images: []ibc.DockerImage{
			PersistenceCoreImage,
		},
	}

	return config
}

func CreateChain(
	t *testing.T,
	ctx context.Context,
	numVals, numFull int,
	genesisOverrides ...cosmos.GenesisKV,
) (*interchaintest.Interchain, *cosmos.CosmosChain) {
	cf := interchaintest.NewBuiltinChainFactory(zaptest.NewLogger(t), []*interchaintest.ChainSpec{
		{
			Name:          "persistence",
			ChainName:     "persistence",
			Version:       PersistenceCoreImage.Version,
			ChainConfig:   persistenceChainConfig(genesisOverrides...),
			NumValidators: &numVals,
			NumFullNodes:  &numFull,
		},
	})

	chains, err := cf.Chains(t.Name())
	require.NoError(t, err)

	ic := interchaintest.NewInterchain().AddChain(chains[0])
	client, network := interchaintest.DockerSetup(t)

	err = ic.Build(
		ctx,
		testreporter.NewNopReporter().RelayerExecReporter(t),
		interchaintest.InterchainBuildOptions{
			TestName:         t.Name(),
			Client:           client,
			NetworkID:        network,
			SkipPathCreation: true,
		},
	)
	require.NoError(t, err)

	return ic, chains[0].(*cosmos.CosmosChain)
}
