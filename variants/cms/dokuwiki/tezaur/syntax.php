<?php
/**
 * <tezaur>question</tezaur>            cited answer
 * <tezaur compare>question</tezaur>    compare sources     (also: gaps)
 * <tezaur search>terms</tezaur>        matching passages only (no model)
 */
class syntax_plugin_tezaur extends DokuWiki_Syntax_Plugin {
    public function getType()  { return 'substition'; }
    public function getPType() { return 'block'; }
    public function getSort()  { return 160; }

    public function connectTo($mode) {
        $this->Lexer->addSpecialPattern('<tezaur(?: (?:compare|gaps|search))?>.*?</tezaur>', $mode, 'plugin_tezaur');
    }

    public function handle($match, $state, $pos, Doku_Handler $handler) {
        preg_match('/^<tezaur(?: (compare|gaps|search))?>(.*?)<\/tezaur>$/s', $match, $m);
        return array($m[1] !== '' ? $m[1] : 'ask', trim($m[2]));
    }

    public function render($format, Doku_Renderer $R, $data) {
        if ($format !== 'xhtml') return false;
        list($mode, $q) = $data;
        $R->nocache(); // answers depend on the library, never cache the page
        $t = plugin_load('helper', 'tezaur');
        $out = '<div class="plugin_tezaur">';
        if ($q === '') {
            $out .= '<p>' . hsc('Empty question.') . '</p>';
        } elseif ($mode === 'search') {
            $r = $t->search($q);
            if (isset($r['error'])) $out .= '<p class="error">' . hsc($r['error']) . '</p>';
            else foreach ($r['hits'] as $h) {
                $out .= '<blockquote><p>' . hsc(mb_substr($h['text'], 0, 400)) . '</p><p><em>[' . (int)$h['n'] . '] ' . hsc($h['label']) . '</em></p></blockquote>';
            }
        } else {
            $r = $t->ask($q, $mode);
            if (isset($r['error'])) {
                $out .= '<p class="error">' . hsc($r['error']) . '</p>';
            } else {
                foreach (preg_split('/\n{2,}/', trim($r['answer'])) as $para) $out .= '<p>' . hsc($para) . '</p>';
                if (!empty($r['references'])) $out .= '<pre class="tezaur-refs">' . hsc($r['references']) . '</pre>';
                foreach ($r['warnings'] as $w) $out .= '<p class="error">&#9888; ' . hsc($w) . '</p>';
            }
        }
        $R->doc .= $out . '</div>';
        return true;
    }
}
