import { ApiPromise } from '@polkadot/api';
import web3 from 'web3';

export const typedefs = {
    AccountId: 'EthereumAccountId',
    AccountId20: 'EthereumAccountId',
    AccountId32: 'EthereumAccountId',
    Address: 'AccountId',
    LookupSource: 'AccountId',
    Lookup0: 'AccountId',
    EthereumSignature: {
      r: 'H256',
      s: 'H256',
      v: 'U8'
    },
    ExtrinsicSignature: 'EthereumSignature',
    SessionKeys: '([u8; 32], [u8; 32])'
  };
  
  export const FEE_PROXY_ADDRESS = '0x00000000000000000000000000000000000004bb';
  
  export const FEE_PROXY_ABI = [
    'function callWithFeePreferences(address asset, uint128 maxPayment, address target, bytes input)',
  ];
  
  export const ERC20_ABI = [
    'event Transfer(address indexed from, address indexed to, uint256 value)',
    'event Approval(address indexed owner, address indexed spender, uint256 value)',
    'function approve(address spender, uint256 amount) public returns (bool)',
    'function allowance(address owner, address spender) public view returns (uint256)',
    'function balanceOf(address who) public view returns (uint256)',
    'function name() public view returns (string memory)',
    'function symbol() public view returns (string memory)',
    'function decimals() public view returns (uint8)',
    'function transfer(address who, uint256 amount)',
  ];
  
  export const NATIVE_TOKEN_ID = 1;
  
  export const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));
  
  export const assetIdToERC20ContractAddress = (assetId: string | Number): string => {
    const asset_id_hex = (+assetId).toString(16).padStart(8, '0');
    return web3.utils.toChecksumAddress(`0xCCCCCCCC${asset_id_hex}000000000000000000000000`);
  }
  

/**
 * Fields of a Polkadotjs event to match on
 */
interface EventMatchers {
    /**
     * Method of a pallet being matched on
     */
    method?: string,
    /**
     * Section, or pallet to match on
     */
    section?: string
}

/**
 * Match on some amount of previous polkadotjs events up to `previousBlocks` behind, executing `fn` on any event results
 * WARNING: use for tests only, as this makes use of the `events()` storage item
 */
export const executeForPreviousEvent = async (
    api: ApiPromise,
    /**
     * Matchers with fields that will match one or more event being queried for
     */
    matchers: EventMatchers,
    /**
     * Maximum number of blocks to check in history
     */
    previousBlocks: number,
    /**
     * Callback to execute on a found event, given the event data.
     */
    fn: (retrievedEventData: any) => any
) => {
    const currentHash = await api.rpc.chain.getBlockHash();
    let parentHash: any = currentHash;

    let currentInHistory = 0;
    while (currentInHistory !== previousBlocks) {
        let events;
        if (parentHash === null) {
            events = await api.query.system.events();
            // Set initial parentHash
            parentHash = await api.query.system.parentHash();

        } else {
            events = await api.query.system.events.at(parentHash);
            // new parentHash for next iteration
            parentHash = await api.query.system.parentHash.at(parentHash);
        }

        events.forEach(async ({ event }: { event: any } ) => {
            // For any events, only match on combination of matchers, or single matcher
            if ('method' in matchers && 'section' in matchers) {
                if (event.method === matchers.method && event.section === matchers.section) {
                    await fn(event);
                }
            } else if ('method' in matchers && matchers.method === event.method) {
                await fn(event);
            } else if ('section' in matchers && matchers.section === event.section) {
                await fn(event);
            }
        });
        currentInHistory++;
    }
}
