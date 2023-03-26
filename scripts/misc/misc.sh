#!/bin/sh
set -e

arg_exists () {
    SEARCH_KEYWORD=$1
    shift
    
    for var in "$@"
    do
        if [ "$SEARCH_KEYWORD" = "$var" ]; then
            echo 0
            exit 0
        fi
    done
    
    echo 1
}

arg_value () {
    SEARCH_KEYWORD=$1
    shift;
    
    KEYWORD_FOUND=false
    RESULT=""
    for value in "$@"
    do
        if [ "$KEYWORD_FOUND" = true ] && ! [ "$value" = "${value#-}" ]; then
            break
        fi
        if [ "$KEYWORD_FOUND" = true ]; then
            if [ -z "$RESULT" ]; then
                RESULT="$value"
            else
                RESULT="$RESULT $value"
            fi
        fi
        if [ "$SEARCH_KEYWORD" = "$value" ]; then
            KEYWORD_FOUND=true
        fi
    done
    
    echo "$RESULT"
}

get_network () {
    determine_network () {
        if [ "$LOCAL" = "0" ]; then
            NETWORK="local"
            elif [ "$PORCINI" = "0" ]; then
            NETWORK="porcini"
            elif [ "$ROOT" = "0" ]; then
            NETWORK="root"
        fi
    }
    
    # Initialize variables
    NETWORK=""
    LOCAL=$(arg_exists --local $@)
    PORCINI=$(arg_exists --porcini $@)
    ROOT=$(arg_exists --root $@)
    DEFAULT=$(arg_value --default $@)
    
    determine_network
    
    # Check if no network was determined and default value is present
    if [ -z "$NETWORK" ] && ! [ -z "$DEFAULT" ]; then
        # Set network based on default value
        case "$DEFAULT" in
            local)      LOCAL=0;;
            porcini)    PORCINI=0;;
            root)       ROOT=0;;
        esac
        
        determine_network
    fi
    
    echo "$NETWORK"
}

get_uri () {
    determine_uri () {
        if [ "$LOCAL" = "0" ]; then
            URI="$LOCAL_URI_RPC"
            if [ "$WSS" = "0" ]; then
                URI="$LOCAL_URI_WSS"
            fi
        fi
        
        if [ "$PORCINI" = "0" ]; then
            URI="$PORCINI_URI_RPC"
            if [ "$WSS" = "0" ]; then
                URI="$PORCINI_URI_WSS"
            fi
        fi
        
        if [ "$ROOT" = "0" ]; then
            URI="$ROOT_URI_RPC"
            if [ "$WSS" = "0" ]; then
                URI="$ROOT_URI_WSS"
            fi
        fi
    }
    
    PORCINI_URI_RPC="https://porcini.au.rootnet.app/archive"
    PORCINI_URI_WSS="wss://porcini.au.rootnet.app:443/archive/ws"
    ROOT_URI_RPC="https://root.au.rootnet.live/archive"
    ROOT_URI_WSS="wss://root.au.rootnet.live:443/archive/ws"
    LOCAL_URI_RPC="http://127.0.0.1:9933"
    LOCAL_URI_WSS="ws://127.0.0.1:9944"
    
    URI=""
    RPC=$(arg_exists --rpc $@)
    WSS=$(arg_exists --wss $@)
    LOCAL=$(arg_exists --local $@)
    PORCINI=$(arg_exists --porcini $@)
    ROOT=$(arg_exists --root $@)
    DEFAULT=$(arg_value --default $@)
    
    determine_uri
    
    if [ -z "$URI" ] && ! [ -z "$DEFAULT" ]; then
        case "$DEFAULT" in
            local)      LOCAL=0;;
            porcini)    PORCINI=0;;
            root)       ROOT=0;;
        esac
        
        determine_uri
    fi
    
    echo "$URI"
}

SUBCOMMAND=$1
shift;

case "$SUBCOMMAND" in
    arg_exists)     arg_exists "$@";;
    arg_value)      arg_value "$@";;
    get_uri)        get_uri "$@";;
    get_network)    get_network "$@";;
    *)              exit 1
esac
