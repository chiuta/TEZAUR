;;; tezaur.el --- Cited answers from your own library (TEZAUR)  -*- lexical-binding: t; -*-
;; Version: 0.1.0
;; Package-Requires: ((emacs "27.1"))
;;; Commentary:
;; M-x tezaur-ask / tezaur-compare / tezaur-gaps / tezaur-search; M-x tezaur-insert pastes the last answer.
;; Needs a running `tezaur serve'.  UNTESTED in a real Emacs (written without one available).
;;; Code:
(require 'url)
(require 'json)

(defgroup tezaur nil "TEZAUR research assistant." :group 'tools)
(defcustom tezaur-server "http://127.0.0.1:8787" "Address of `tezaur serve'." :type 'string)
(defcustom tezaur-token nil "Access token, or nil." :type '(choice (const nil) string))
(defcustom tezaur-k 6 "Passages per question." :type 'integer)

(defvar tezaur--last nil "Text of the last answer.")

(defun tezaur--format (mode d)
  "Format decoded JSON D (an alist) for MODE as a string."
  (let ((err (alist-get 'error d)))
    (if err (concat "Error: " err)
      (if (equal mode "search")
          (mapconcat (lambda (h) (format "[%d] %s\n    %s" (alist-get 'n h) (alist-get 'label h)
                                         (replace-regexp-in-string "[ \n]+" " " (substring (alist-get 'text h) 0 (min 300 (length (alist-get 'text h)))))))
                     (alist-get 'hits d) "\n")
        (concat (alist-get 'answer d)
                (let ((r (alist-get 'references d))) (if (and r (not (string-empty-p r))) (concat "\n\n## References\n" r) ""))
                (mapconcat (lambda (w) (concat "\n⚠ " w)) (alist-get 'warnings d) ""))))))

(defun tezaur--run (mode q)
  (when (string-empty-p (string-trim q)) (user-error "Empty question"))
  (message "TEZAUR: thinking… (a local model can take a minute)")
  (let* ((url-request-method "POST")
         (url-request-extra-headers
          (append '(("Content-Type" . "application/json"))
                  (when tezaur-token `(("Authorization" . ,(concat "Bearer " tezaur-token))))))
         (url-request-data (encode-coding-string (json-encode `((q . ,q) (mode . ,mode) (k . ,tezaur-k))) 'utf-8))
         (buf (url-retrieve-synchronously (concat (string-remove-suffix "/" tezaur-server) (if (equal mode "search") "/api/search" "/api/ask")) t t 600)))
    (unless buf (user-error "TEZAUR: cannot reach %s" tezaur-server))
    (with-current-buffer buf
      (goto-char (point-min))
      (re-search-forward "\n\n" nil t)
      (let* ((d (condition-case nil (json-parse-buffer :object-type 'alist :array-type 'list) (error '((error . "unexpected answer")))))
             (text (tezaur--format mode d)))
        (kill-buffer buf)
        (setq tezaur--last text)
        (with-current-buffer (get-buffer-create "*TEZAUR*")
          (let ((inhibit-read-only t)) (erase-buffer) (insert text) (goto-char (point-min)))
          (when (fboundp 'markdown-mode) (markdown-mode))
          (special-mode)
          (display-buffer (current-buffer)))))))

;;;###autoload
(defun tezaur-ask (q) "Ask the library Q." (interactive (list (if (use-region-p) (buffer-substring-no-properties (region-beginning) (region-end)) (read-string "Ask TEZAUR: ")))) (tezaur--run "ask" q))
;;;###autoload
(defun tezaur-compare (q) "Compare sources on Q." (interactive "sCompare sources on: ") (tezaur--run "compare" q))
;;;###autoload
(defun tezaur-gaps (q) "Show gaps on Q." (interactive "sGaps on: ") (tezaur--run "gaps" q))
;;;###autoload
(defun tezaur-search (q) "Search passages for Q (no model)." (interactive "sSearch passages: ") (tezaur--run "search" q))
;;;###autoload
(defun tezaur-insert () "Insert the last answer at point." (interactive) (if tezaur--last (insert tezaur--last) (user-error "Nothing to insert yet")))

(provide 'tezaur)
;;; tezaur.el ends here
