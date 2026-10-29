<?php
// Stub harness: just enough of DokuWiki / WordPress to run the plugin code against a real `tezaur serve`.
$server = getenv('TEZAUR_SERVER') ?: 'http://127.0.0.1:8810';
function hsc($s) { return htmlspecialchars($s, ENT_QUOTES); }
class DokuWiki_Plugin { function getConf($k) { global $server; return ['server'=>$server,'token'=>'','k'=>4,'workspace'=>''][$k]; } }
class DokuWiki_Syntax_Plugin extends DokuWiki_Plugin { public $Lexer; }
class Doku_Handler {} class Doku_Renderer { public $doc=''; public $nc=false; function nocache(){ $this->nc=true; } }
class DokuHTTPClient { public $timeout; public $headers=[]; function get($u){ return @file_get_contents($u); } }
$helper = null; function plugin_load($t,$n){ global $helper; return $helper; }
require __DIR__.'/../dokuwiki/tezaur/helper.php'; require __DIR__.'/../dokuwiki/tezaur/syntax.php';
$helper = new helper_plugin_tezaur(); $syn = new syntax_plugin_tezaur();
function check($c,$m){ if(!$c){ fwrite(STDERR,"FAIL: $m\n"); exit(1);} echo "ok  $m\n"; }
$d = $syn->handle('<tezaur compare>burden of proof</tezaur>', 0, 0, new Doku_Handler());
check($d === ['compare','burden of proof'], 'dokuwiki handle parses mode+question');
$R = new Doku_Renderer(); $syn->render('xhtml', $R, ['ask','burden of proof']);
check(strpos($R->doc,'burden of proof lies')!==false && $R->nc, 'dokuwiki ask renders answer and disables cache');
check(strpos($R->doc,'tezaur-refs')!==false && strpos($R->doc,'&#9888;')!==false, 'dokuwiki shows references and warnings');
$R = new Doku_Renderer(); $syn->render('xhtml', $R, ['search','burden']);
check(strpos($R->doc,'<blockquote>')!==false, 'dokuwiki search renders passages');
$server = 'http://127.0.0.1:1'; $R = new Doku_Renderer(); $syn->render('xhtml', $R, ['ask','x']);
check(strpos($R->doc,'cannot reach')!==false, 'dokuwiki reports unreachable server');
$server = getenv('TEZAUR_SERVER') ?: 'http://127.0.0.1:8810';

// ---- WordPress
function add_shortcode(){} function add_action(){} function register_rest_route(){} function shortcode_atts($a,$b){return array_merge($a,$b);}
$opts = ['server'=>$server,'k'=>4]; $logged = false; $trans = [];
function get_option($k,$d=null){ global $opts; return ['tezaur_options'=>$opts][$k] ?? $d; }
function is_wp_error($x){ return false; } function wp_remote_retrieve_body($r){ return $r; }
function wp_remote_get($u,$a){ return @file_get_contents($u) ?: '{}'; }
function current_user_can(){ global $logged; return $logged; } function is_user_logged_in(){ global $logged; return $logged; }
function get_transient($k){ global $trans; return $trans[$k] ?? false; } function set_transient($k,$v){ global $trans; $trans[$k]=$v; }
class WP_REST_Response { public $data; public $status; function __construct($d,$s=200){ $this->data=$d; $this->status=$s; } }
class Req { function __construct(public $p){} function get_param($k){ return $this->p[$k] ?? null; } }
define('ABSPATH', '/'); require __DIR__.'/../wordpress/tezaur-research/tezaur-research.php';
$r = tezaur_rest_ask(new Req(['q'=>'x']));
check($r->status === 403, 'wordpress refuses anonymous visitors by default');
$logged = true; $r = tezaur_rest_ask(new Req(['q'=>'burden of proof','mode'=>'ask']));
check(isset($r->data['answer']) && $r->status===200, 'wordpress ask returns answer');
$r = tezaur_rest_ask(new Req(['q'=>'burden','mode'=>'search']));
check(!empty($r->data['hits']), 'wordpress search returns hits');
check(tezaur_rest_ask(new Req(['q'=>'']))->status === 400, 'wordpress rejects empty question');
$logged = false; $opts['public'] = 1; $opts['rate'] = 2; $_SERVER['REMOTE_ADDR']='1.2.3.4';
$ok = [tezaur_allowed(), tezaur_allowed(), tezaur_allowed()];
check($ok === [true,true,false], 'wordpress public mode rate-limits per IP');
echo "all php tests passed\n";
