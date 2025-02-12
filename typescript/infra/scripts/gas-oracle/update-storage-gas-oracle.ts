import { ChainName, HyperlaneIgp, MultiProvider } from '@hyperlane-xyz/sdk';
import { ProtocolType } from '@hyperlane-xyz/utils';

import { RemoteGasData, StorageGasOracleConfig } from '../../src/config';
import { deployEnvToSdkEnv } from '../../src/config/environment';
import { RemoteGasDataConfig } from '../../src/config/gas-oracle';
import { getArgs, getEnvironmentConfig, withNetwork } from '../utils';

import {
  eqRemoteGasData,
  prettyRemoteGasData,
  prettyRemoteGasDataConfig,
} from './utils';

/**
 * Idempotent. Use `--dry-run` to not send any transactions.
 * Updates the currently stored gas data on the StorageGasOracle contract
 * if the configured data differs from the on-chain data.
 * Expects the deployer key to be the owner of the StorageGasOracle contract.
 */
async function main() {
  const args = await withNetwork(getArgs())
    .boolean('dry-run')
    .describe('dry-run', 'If true, will not submit any transactions')
    .default('dry-run', false).argv;

  const { environment, network } = args;
  const coreEnvConfig = getEnvironmentConfig(environment);
  const multiProvider = await coreEnvConfig.getMultiProvider();

  const storageGasOracleConfig = coreEnvConfig.storageGasOracleConfig;
  if (!storageGasOracleConfig) {
    throw Error(`No storage gas oracle config for environment ${environment}`);
  }

  const igp = HyperlaneIgp.fromEnvironment(
    deployEnvToSdkEnv[environment],
    multiProvider,
  );

  const targetChains = network ? [network] : igp.chains();

  for (const chain of targetChains) {
    if (
      multiProvider.getChainMetadata(chain).protocol !== ProtocolType.Ethereum
    ) {
      console.log(`Skipping ${chain} because it is not an Ethereum chain`);
      continue;
    }

    await setStorageGasOracleValues(
      igp,
      multiProvider,
      storageGasOracleConfig[chain],
      chain,
      args.dryRun,
    );
    console.log('\n===========');
  }
}

async function setStorageGasOracleValues(
  igp: HyperlaneIgp,
  // This multiProvider is used instead of the one on the IGP because the IGP's
  // multiprovider will have filtered out non-Ethereum chains.
  multiProvider: MultiProvider,
  localStorageGasOracleConfig: StorageGasOracleConfig,
  local: ChainName,
  dryRun: boolean,
) {
  console.log(`Setting remote gas data on local chain ${local}...`);
  const storageGasOracle = igp.getContracts(local).storageGasOracle;

  const configsToSet: RemoteGasDataConfig[] = [];

  for (const remote in localStorageGasOracleConfig) {
    const desiredGasData = localStorageGasOracleConfig[remote]!;
    const remoteId = multiProvider.getDomainId(remote);

    const existingGasData: RemoteGasData = await storageGasOracle.remoteGasData(
      remoteId,
    );

    console.log(
      `${local} -> ${remote} existing gas data:\n`,
      prettyRemoteGasData(existingGasData),
    );
    console.log(
      `${local} -> ${remote} desired gas data:\n`,
      prettyRemoteGasData(desiredGasData),
    );

    if (eqRemoteGasData(existingGasData, desiredGasData)) {
      console.log('Existing and desired gas data are the same, doing nothing');
    } else {
      console.log('Existing and desired gas data differ, will update');
      configsToSet.push({
        remoteDomain: remoteId,
        ...desiredGasData,
      });
    }
    console.log('---');
  }

  if (configsToSet.length > 0) {
    console.log(`Updating ${configsToSet.length} configs on local ${local}:`);
    console.log(
      configsToSet
        .map((config) => prettyRemoteGasDataConfig(multiProvider, config))
        .join('\n\t--\n'),
    );

    if (dryRun) {
      console.log('Running in dry run mode, not sending tx');
    } else {
      await igp.multiProvider.sendTransaction(
        local,
        await storageGasOracle.populateTransaction.setRemoteGasDataConfigs(
          configsToSet,
        ),
      );
    }
  }
}

main().catch((err) => console.error('Error', err));
