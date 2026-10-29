<?php
use MediaWiki\MediaWikiServices;

class TezaurHooks {
    public static function onParserFirstCallInit( Parser $parser ) {
        $parser->setHook( 'tezaur', [ self::class, 'render' ] );
    }

    public static function render( $input, array $args, Parser $parser, PPFrame $frame ) {
        $parser->getOutput()->updateCacheExpiry( 0 ); // answers depend on the library: do not cache
        $q = trim( $input );
        if ( $q === '' ) { return ''; }
        $cfg = MediaWikiServices::getInstance()->getMainConfig();
        $mode = $args['mode'] ?? 'ask';
        $path = $mode === 'search' ? '/api/search' : '/api/ask';
        $url = rtrim( $cfg->get( 'TezaurServer' ), '/' ) . $path . '?' . http_build_query( [
            'q' => $q, 'k' => $cfg->get( 'TezaurK' ),
            'mode' => in_array( $mode, [ 'compare', 'gaps' ], true ) ? $mode : 'ask',
        ] );
        $opts = [ 'timeout' => 300 ];
        $req = MediaWikiServices::getInstance()->getHttpRequestFactory()->create( $url, $opts, __METHOD__ );
        if ( $cfg->get( 'TezaurToken' ) !== '' ) { $req->setHeader( 'Authorization', 'Bearer ' . $cfg->get( 'TezaurToken' ) ); }
        $status = $req->execute();
        $d = $status->isOK() ? json_decode( $req->getContent(), true ) : null;
        $e = static function ( $s ) { return htmlspecialchars( (string)$s, ENT_QUOTES ); };
        if ( !is_array( $d ) || isset( $d['error'] ) ) {
            return '<p class="error">' . $e( $d['error'] ?? 'cannot reach the TEZAUR server' ) . '</p>';
        }
        $out = '<div class="tezaur">';
        if ( isset( $d['answer'] ) ) {
            foreach ( preg_split( '/\n{2,}/', trim( $d['answer'] ) ) as $p ) { $out .= '<p>' . $e( $p ) . '</p>'; }
            if ( !empty( $d['references'] ) ) { $out .= '<pre>' . $e( $d['references'] ) . '</pre>'; }
            foreach ( $d['warnings'] as $w ) { $out .= '<p class="error">&#9888; ' . $e( $w ) . '</p>'; }
        } else {
            foreach ( $d['hits'] as $h ) {
                $out .= '<blockquote>' . $e( mb_substr( $h['text'], 0, 400 ) ) . '<br><i>[' . (int)$h['n'] . '] ' . $e( $h['label'] ) . '</i></blockquote>';
            }
        }
        return $out . '</div>';
    }
}
