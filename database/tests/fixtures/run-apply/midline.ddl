-- A DDL file that ends the transaction mid-line, where no whole-line guard
-- sees it. --apply must detect the ended transaction and fail naming the file
-- (L3). rollback (not commit), so nothing persists while detection is missing.
select 1; rollback;
