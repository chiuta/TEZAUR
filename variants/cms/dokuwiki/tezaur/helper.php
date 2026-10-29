<?php
/**
 * TEZAUR helper: talks to `tezaur serve` over HTTP. Used by the syntax plugin; other plugins can call
 *   $t = plugin_load('helper', 'tezaur'); $t->ask('question');
 */
class helper_plugin_tezaur extends DokuWiki_Plugin {

    /** @return array decoded JSON, or ['error' => message] */
    public function call($path, $params = array()) {
        $params['k'] = $this->getConf('k');
        if ($this->getConf('workspace') !== '') $params['ws'] = $this->getConf('workspace');
        $url = rtrim($this->getConf('server'), '/') . $path . '?' . http_build_query($params);
        $http = new DokuHTTPClient();
        $http->timeout = 300; // local models are slow
        if ($this->getConf('token') !== '') $http->headers['Authorization'] = 'Bearer ' . $this->getConf('token');
        $body = $http->get($url);
        if ($body === false) return array('error' => 'cannot reach the TEZAUR server (' . $this->getConf('server') . ')');
        $data = json_decode($body, true);
        return is_array($data) ? $data : array('error' => 'unexpected answer from the TEZAUR server');
    }

    public function ask($q, $mode = 'ask')  { return $this->call('/api/ask',    array('q' => $q, 'mode' => $mode)); }
    public function search($q)              { return $this->call('/api/search', array('q' => $q)); }
}
