<?php
/**
 * Plugin Name: TEZAUR Research
 * Description: Ask your own library from WordPress: [tezaur_ask] and [tezaur_search] shortcodes backed by a `tezaur serve` instance. The access token stays on the server side.
 * Version: 0.1.0
 * License: MIT
 */
if (!defined('ABSPATH')) exit;

function tezaur_opt($k, $d = '') { $o = get_option('tezaur_options', array()); return isset($o[$k]) && $o[$k] !== '' ? $o[$k] : $d; }

/** Calls the TEZAUR server. Returns decoded JSON or array('error' => ...). */
function tezaur_call($path, $params) {
    $params['k'] = (int) tezaur_opt('k', 6);
    if (tezaur_opt('workspace')) $params['ws'] = tezaur_opt('workspace');
    $url = rtrim(tezaur_opt('server', 'http://127.0.0.1:8787'), '/') . $path . '?' . http_build_query($params);
    $headers = array();
    if (tezaur_opt('token')) $headers['Authorization'] = 'Bearer ' . tezaur_opt('token');
    $r = wp_remote_get($url, array('timeout' => 300, 'headers' => $headers));
    if (is_wp_error($r)) return array('error' => $r->get_error_message());
    $d = json_decode(wp_remote_retrieve_body($r), true);
    return is_array($d) ? $d : array('error' => 'unexpected answer from the TEZAUR server');
}

/** Visitors may only use it when "public" is on; then a small per-IP limit protects your (slow, local) model. */
function tezaur_allowed() {
    if (current_user_can('read') && is_user_logged_in()) return true;
    if (!tezaur_opt('public')) return false;
    $key = 'tezaur_rl_' . md5(isset($_SERVER['REMOTE_ADDR']) ? $_SERVER['REMOTE_ADDR'] : 'x');
    $n = (int) get_transient($key);
    if ($n >= (int) tezaur_opt('rate', 6)) return false;
    set_transient($key, $n + 1, 60);
    return true;
}

function tezaur_rest_ask($req) {
    if (!tezaur_allowed()) return new WP_REST_Response(array('error' => 'not allowed (log in, or ask the site owner to enable public use)'), 403);
    $q = trim((string) $req->get_param('q'));
    if ($q === '') return new WP_REST_Response(array('error' => 'missing q'), 400);
    $mode = $req->get_param('mode');
    if ($mode === 'search') return new WP_REST_Response(tezaur_call('/api/search', array('q' => $q)));
    if (!in_array($mode, array('ask', 'compare', 'gaps'), true)) $mode = 'ask';
    return new WP_REST_Response(tezaur_call('/api/ask', array('q' => $q, 'mode' => $mode)));
}

add_action('rest_api_init', function () {
    register_rest_route('tezaur/v1', '/ask', array('methods' => 'GET,POST', 'callback' => 'tezaur_rest_ask', 'permission_callback' => '__return_true'));
});

function tezaur_shortcode($atts, $content, $tag) {
    $a = shortcode_atts(array('mode' => $tag === 'tezaur_search' ? 'search' : 'ask', 'placeholder' => 'Ask the library…'), $atts);
    $id = 'tz' . wp_rand(1000, 9999);
    $rest = esc_url_raw(rest_url('tezaur/v1/ask'));
    $nonce = wp_create_nonce('wp_rest');
    ob_start(); ?>
    <div class="tezaur" id="<?php echo esc_attr($id); ?>">
      <form><input type="text" style="width:70%" placeholder="<?php echo esc_attr($a['placeholder']); ?>"> <button type="submit">Go</button></form>
      <div class="tezaur-out" aria-live="polite"></div>
    </div>
    <script>
    (function(){var r=document.getElementById(<?php echo wp_json_encode($id); ?>),f=r.querySelector('form'),o=r.querySelector('.tezaur-out');
    f.addEventListener('submit',function(e){e.preventDefault();var q=f.querySelector('input').value.trim();if(!q)return;o.textContent='Thinking… (a local model can take a minute)';
    fetch(<?php echo wp_json_encode($rest); ?>+'?mode='+encodeURIComponent(<?php echo wp_json_encode($a['mode']); ?>)+'&q='+encodeURIComponent(q),{headers:{'X-WP-Nonce':<?php echo wp_json_encode($nonce); ?>}}).then(function(x){return x.json()}).then(function(d){
      o.textContent='';function p(t,c){var e=document.createElement(c||'p');e.textContent=t;o.appendChild(e);return e}
      if(d.error)return p(d.error);
      if(d.answer){d.answer.split(/\n{2,}/).forEach(function(t){p(t)});if(d.references)p(d.references,'pre');(d.warnings||[]).forEach(function(w){p('⚠ '+w)})}
      else (d.hits||[]).forEach(function(h){var b=p(h.text.slice(0,400),'blockquote');var m=document.createElement('cite');m.textContent=' ['+h.n+'] '+h.label;b.appendChild(m)});
    }).catch(function(e){o.textContent=String(e)})});})();
    </script>
    <?php return ob_get_clean();
}
add_shortcode('tezaur_ask', 'tezaur_shortcode');
add_shortcode('tezaur_search', 'tezaur_shortcode');

add_action('admin_menu', function () {
    add_options_page('TEZAUR', 'TEZAUR', 'manage_options', 'tezaur', function () { ?>
      <div class="wrap"><h1>TEZAUR</h1><form method="post" action="options.php">
        <?php settings_fields('tezaur'); $o = get_option('tezaur_options', array()); ?>
        <table class="form-table">
        <tr><th>Server</th><td><input name="tezaur_options[server]" value="<?php echo esc_attr(isset($o['server']) ? $o['server'] : 'http://127.0.0.1:8787'); ?>" class="regular-text"></td></tr>
        <tr><th>Token</th><td><input type="password" name="tezaur_options[token]" value="<?php echo esc_attr(isset($o['token']) ? $o['token'] : ''); ?>" class="regular-text"></td></tr>
        <tr><th>Workspace</th><td><input name="tezaur_options[workspace]" value="<?php echo esc_attr(isset($o['workspace']) ? $o['workspace'] : ''); ?>"></td></tr>
        <tr><th>Passages</th><td><input type="number" name="tezaur_options[k]" value="<?php echo esc_attr(isset($o['k']) ? $o['k'] : 6); ?>"></td></tr>
        <tr><th>Public use</th><td><label><input type="checkbox" name="tezaur_options[public]" value="1" <?php checked(!empty($o['public'])); ?>> let non-logged-in visitors ask (limited per IP/minute)</label></td></tr>
        <tr><th>Visitor limit / min</th><td><input type="number" name="tezaur_options[rate]" value="<?php echo esc_attr(isset($o['rate']) ? $o['rate'] : 6); ?>"></td></tr>
        </table><?php submit_button(); ?></form></div>
    <?php });
});
add_action('admin_init', function () { register_setting('tezaur', 'tezaur_options'); });
