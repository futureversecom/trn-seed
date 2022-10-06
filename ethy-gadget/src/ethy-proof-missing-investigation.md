## Issue

Ethy worker expects to have event metadata available when storing a proof

Current code can allow reaching consensus on an event without storing the event metadata first
- this is (maybe) invalid since it means the event digest is not verified for consistency

- storing event metadata handled by worker.handle_finality thread
- storing witnesses handled by worker.handle_witness thread

## `gossip_validator.validate`
 checks incoming witnesses from the network
     - public key is an active validator
    - signature are valid for the public key but not that the digest is valid
    - witness is not a duplicate

## `note_event_witness`
further processes witnesses from the gossip validator
    - is the public key an active validator?
    - does the witness digest match our known digest?
    - if digest is not known processes the witness anyway

## `worker handle_finality_notification`
pull ethy signing events/requests out of finalized block headers
- full nodes
    - store metadata

- validators
    - store metadata
    - sign witness
    - broadcast witness


## Ideas

- store witnesses in pending state until event metadata is available
- 
