package interchaintest

type ContractInstantiateMsg struct {
	LsPrefix     string       `json:"ls_prefix"`
	PresetIbcFee PresetIbcFee `json:"preset_ibc_fee"`
	Timeouts     Timeouts     `json:"timeouts"`
}

type QueryLsConfigMsg struct {
	LsConfig struct{} `json:"ls_config"`
}

type Active struct {
	Active bool `json:"active"`
}

type QueryLsConfigResp struct {
	Data Active `json:"data"`
}

type PresetIbcFee struct {
	AckFee     string `json:"ack_fee"`
	TimeoutFee string `json:"timeout_fee"`
}

type Timeouts struct {
	IbcTransferTimeout string `json:"ibc_transfer_timeout"`
	IcaTimeout         string `json:"ica_timeout"`
}
