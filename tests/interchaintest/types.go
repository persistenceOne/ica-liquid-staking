package interchaintest

type ContractInstantiateMsg struct {
	LsPrefix     string       `json:"ls_prefix"`
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
